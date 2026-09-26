use super::types::Node;
use crate::GITHUB_KEY;
use crate::constants::{GH_GRAPH_QL_100_REPOS_FRAGMENT, NEEDS_UPDATE_CHUNK_SIZE};
use libsql::{Connection, params};
use std::error::Error;
use std::sync::Arc;

#[derive(Clone, Debug)]
struct NeedsUpdateRow {
    id: String,
    type_of_repo: String,
}

pub async fn fetch_root_folder_directory_files(
    client: &reqwest::Client,
    user_name: String,
    repo_name: String,
    branch_or_tag: String,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let url = format!(
        "https://api.github.com/repos/{user_name}/{repo_name}/contents?ref={branch_or_tag}"
    );
    let response = client
        .get(&url)
        .header("User-Agent", "zigistry")
        .header("Authorization", &*GITHUB_KEY)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("GitHub API returned {}", response.status()).into());
    }
    let response_json: Vec<serde_json::Value> = response.json().await?;
    let mut directories = Vec::new();
    let mut files = Vec::new();

    for thing in response_json {
        let name = match thing["name"].as_str() {
            Some(name) => name.to_string(),
            None => continue,
        };
        let kind = match thing["type"].as_str() {
            Some(kind) => kind,
            None => continue,
        };

        match kind {
            "dir" => directories.push(name),
            "file" => files.push(name),
            _ => {}
        }
    }
    let dirs_string = directories.join("\n");
    let files_string = files.join("\n");
    let join_both_strings = dirs_string + "\n\n" + &files_string;
    Ok(join_both_strings)
}

fn parse_github_repo_id(repo_id: &str) -> Option<(String, String)> {
    let mut parts = repo_id.split('/');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("gh"), Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty() => {
            Some((owner.to_string(), name.to_string()))
        }
        _ => None,
    }
}

fn escape_graphql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn build_batch_query(rows: &[NeedsUpdateRow]) -> String {
    let mut query = String::from("query {\n");
    for (repo_idx, row) in rows.iter().enumerate() {
        let Some((owner, repo)) = parse_github_repo_id(&row.id) else {
            continue;
        };
        query.push_str(&format!(
            "  repo_{repo_idx}: repository(owner: \"{}\", name: \"{}\") {{ ...RepoFields }}\n",
            escape_graphql_string(&owner),
            escape_graphql_string(&repo),
        ));
    }
    query.push_str("}\n");
    query.push_str(GH_GRAPH_QL_100_REPOS_FRAGMENT);
    query
}

async fn make_update_rows_easy(
    connection: &Connection,
) -> Result<Vec<NeedsUpdateRow>, Box<dyn Error + Send + Sync>> {
    let mut rows = connection
        .query(
            "SELECT id, type_of_repo FROM repo_pipeline_queue WHERE id LIKE 'gh/%' AND status = 'needs_update' ORDER BY queued_at",
            params![],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(NeedsUpdateRow {
            id: row.get(0)?,
            type_of_repo: row.get(1)?,
        });
    }
    Ok(out)
}

async fn process_chunk(
    chunk: &[NeedsUpdateRow],
    connection: Arc<Connection>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn Error>> {
    if chunk.is_empty() {
        return Ok(());
    }

    let query_to_send = serde_json::json!({
        "query": build_batch_query(chunk),
    });

    let response = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "zigistry")
        .header("Authorization", &*GITHUB_KEY)
        .json(&query_to_send)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("gql returned {}", response.status()).into());
    }

    let response_body = response.text().await?;
    let response_json: serde_json::Value = serde_json::from_str(&response_body)?;
    if let Some(errors) = response_json.get("errors") {
        eprintln!("gh gql errors: {errors}");
    }

    let Some(data_obj) = response_json
        .get("data")
        .and_then(|value| value.as_object())
    else {
        return Ok(());
    };

    let mut collected = Vec::new();
    let mut skipped_or_invalid_ids = Vec::new();

    for (repo_idx, row) in chunk.iter().enumerate() {
        if parse_github_repo_id(&row.id).is_none() {
            skipped_or_invalid_ids.push(row.id.clone());
            continue;
        }

        let alias = format!("repo_{repo_idx}");
        let Some(repo_value) = data_obj.get(&alias) else {
            skipped_or_invalid_ids.push(row.id.clone());
            continue;
        };
        if repo_value.is_null() {
            eprintln!(
                "no repo data/update for: {}",
                row.id
            );
            skipped_or_invalid_ids.push(row.id.clone());
            continue;
        }

        let node: Node = match serde_json::from_value(repo_value.clone()) {
            Ok(node) => node,
            Err(error) => {
                eprintln!("Skipping {} because parse failed: {}", row.id, error);
                skipped_or_invalid_ids.push(row.id.clone());
                continue;
            }
        };

        let is_package = row.type_of_repo.eq_ignore_ascii_case("package");
        let data = super::get_repo_data(&node, is_package, client).await;
        collected.push((row.id.clone(), data));
    }

    if collected.is_empty() && skipped_or_invalid_ids.is_empty() {
        return Ok(());
    }

    let transaction = connection.transaction().await?;
    let now_epoch = chrono::Utc::now().timestamp();
    for (repo_id, data) in collected {
        super::persist_repo_data(&transaction, data).await;
        transaction
            .execute(
                "UPDATE repo_pipeline_queue SET status = 'indexed', processed_at = ? WHERE id = ?",
                params![now_epoch, repo_id],
            )
            .await?;
    }
    for repo_id in skipped_or_invalid_ids {
        transaction
            .execute(
                "UPDATE repo_pipeline_queue SET status = 'indexed', processed_at = ? WHERE id = ?",
                params![now_epoch, repo_id],
            )
            .await?;
    }
    transaction.commit().await?;

    Ok(())
}

pub async fn run_cron_update_once(pool: Arc<Connection>) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    match make_update_rows_easy(pool.as_ref()).await {
        Ok(rows) => {
            for chunk in rows.chunks(NEEDS_UPDATE_CHUNK_SIZE) {
                if let Err(error) = process_chunk(chunk, Arc::clone(&pool), &client).await {
                    eprintln!("process_chunk failed: {error}");
                }
            }
        }
        Err(error) => {
            eprintln!("failed to read repo_pipeline_queue: {error}");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_root_folder_directory_files() {
    let client = reqwest::Client::new();
    let result = fetch_root_folder_directory_files(
        &client,
        "zigistry".to_string(),
        "zigistry".to_string(),
        "main".to_string(),
    )
    .await
    .unwrap();
    println!("directory_files output:\n{result}");
}
