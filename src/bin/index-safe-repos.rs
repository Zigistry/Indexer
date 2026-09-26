use libsql::params;
use reqwest::StatusCode;
use zigistry::{
    CODEBERG_KEY, GITHUB_KEY, constants::GH_GRAPH_QL_100_REPOS_FRAGMENT,
    database::connect_to_database,
};

fn make_repo_query(owner_name: &str, repo_name: &str) -> String {
    format!(
        "query {{\n  repository(owner: \"{}\", name: \"{}\") {{ ...RepoFields }}\n}}\n{}",
        owner_name.replace('\\', "\\\\").replace('"', "\\\""),
        repo_name.replace('\\', "\\\\").replace('"', "\\\""),
        GH_GRAPH_QL_100_REPOS_FRAGMENT // Actually, this is a fragment I can use for 1 repo as well.
    )
}

async fn process_codeberg_repo(
    owner_name: &str,
    repo_name: &str,
    client: &reqwest::Client,
    transaction: &libsql::Transaction,
) {
    let res = client
        .get(format!(
            "https://codeberg.org/api/v1/repos/{owner_name}/{repo_name}"
        ))
        .header("Authorization", &*CODEBERG_KEY)
        .header("User-Agent", "zigistry")
        .send()
        .await
        .unwrap();
    if res.status() == StatusCode::NOT_FOUND {
        return;
    }
    if !res.status().is_success() {
        panic!("cb responce problem.");
    }

    let repo: zigistry::codeberg::types::types::Daum = res.json().await.unwrap();

    if !zigistry::codeberg::helper_functions::has_zig_in_top_languages(owner_name, repo_name).await
    {
        return;
    }

    let data = zigistry::codeberg::get_repo_data(repo).await;

    zigistry::codeberg::send_repo_data_to_database(&transaction, data).await;
    println!("processed {owner_name}/{repo_name}");
}

async fn process_github_repo(
    owner_name: &str,
    repo_name: &str,
    type_of_repo: &str,
    client: &reqwest::Client,
    transaction: &libsql::Transaction,
) {
    let query = make_repo_query(owner_name, repo_name);
    let body_content = serde_json::json!({ "query": query });
    let res = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "zigistry")
        .header("Authorization", &*GITHUB_KEY)
        .json(&body_content)
        .send()
        .await
        .unwrap();

    if !res.status().is_success() {
        return;
    }

    let json_text = res.text().await.unwrap();

    println!("{json_text}");

    let json: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let Some(data) = json.get("data").and_then(|v| v.as_object()) else {
        eprintln!(
            "GitHub GQL error or data is missing for the {owner_name}/{repo_name} repo \nJSON DATA:\n{json_text}"
        );
        return;
    };

    let val = &data["repository"];

    if val.is_null() {
        println!(
            "Repository {}/{} not found or has been deleted, skipping",
            owner_name, repo_name
        );
        return;
    }

    let repo_node: zigistry::github::types::Node = serde_json::from_value(val.clone()).unwrap();

    if !zigistry::github::has_zig_in_top_languages(&repo_node) {
        return;
    }
    let repo_data =
        zigistry::github::get_repo_data(&repo_node, type_of_repo.eq_ignore_ascii_case("package"), client).await;

    zigistry::github::persist_repo_data(&transaction, repo_data).await;

    println!("{:?}", repo_node);
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let connection = connect_to_database().await.unwrap();

    let transaction = connection.transaction().await.unwrap();

    let mut rows_to_process = transaction
        .query(
            "SELECT id, type_of_repo FROM repo_pipeline_queue WHERE status = 'safe' ORDER BY queued_at",
            params![],
        )
        .await
        .unwrap();

    let mut my_vec = Vec::new();

    while let Some(repo_row) = rows_to_process.next().await.unwrap() {
        let repo_id: String = repo_row.get(0).unwrap();
        let repo_type: String = repo_row.get(1).unwrap();
        my_vec.push((repo_id, repo_type));
    }

    transaction.commit().await.unwrap();

    for repo_row in my_vec {
        let transaction = connection.transaction().await.unwrap();
        let id: String = repo_row.0;
        let type_of_repo: String = repo_row.1;

        let mut parts = id.split('/');

        let platform = parts.next().unwrap();
        let owner_name = parts.next().unwrap();
        let repo_name = parts.next().unwrap();

        if platform == "gh" {
            process_github_repo(owner_name, repo_name, &type_of_repo, &client, &transaction).await;
        } else if platform == "cb" {
            process_codeberg_repo(owner_name, repo_name, &client, &transaction).await;
        } else {
            panic!("got an unknown platform. {}", id)
        }

        let now_epoch = chrono::Utc::now().timestamp();
        transaction
            .execute(
                "UPDATE repo_pipeline_queue SET status = 'indexed', processed_at = ? WHERE id = ?",
                params![now_epoch, id],
            )
            .await
            .unwrap();
        transaction.commit().await.unwrap();
    }
}
