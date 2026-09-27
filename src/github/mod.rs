pub mod cron_update_helper;
pub mod github_data;
pub mod types;
use crate::bzz_stuff::{parse, tokenize};
use crate::constants::limits;
use crate::constants::{ASYNC_LIMIT, GH_GRAPH_QL_PARTIAL_QUERY, GH_GRAPH_QL_QUERY};
use crate::database::{
    parse_lazy_flag, truncate_option_to_char_limit, truncate_to_char_limit,
};
use crate::github::github_data::{ReleaseData, RepoData};
use crate::github::types::{DefaultBranchRef, Node};
use crate::{GITHUB_KEY, custom_types};
use chrono::{Days, NaiveDateTime};
pub use cron_update_helper::run_cron_update_once;
use futures::stream;
use futures::stream::StreamExt;
use libsql::{Connection, Transaction, params};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

fn parse_iso_to_epoch(date_str: &str) -> i64 {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return dt.timestamp();
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%SZ") {
        return naive.and_utc().timestamp();
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return naive.and_utc().timestamp();
    }
    0
}

const EMPTY_REPLY: &str =
    r#"{"data":{"search":{"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]}}}"#;

fn makeing_commit_hash_normal(raw_oid: Option<&str>) -> String {
    raw_oid
        .map(str::trim)
        .filter(|oid| !oid.is_empty())
        .map(|oid| truncate_to_char_limit(oid, limits::REPO_COMMIT_HASH_MAX_LEN))
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_commit_hash(default_branch_ref: Option<&DefaultBranchRef>) -> String {
    let raw_oid = default_branch_ref
        .and_then(|branch| branch.target.as_ref())
        .and_then(|target| target.oid.as_deref());

    makeing_commit_hash_normal(raw_oid)
}

fn get_commit_hash_value(node: &serde_json::Value) -> String {
    let raw_oid = node
        .pointer("/defaultBranchRef/target/oid")
        .and_then(|oid| oid.as_str())
        .or_else(|| {
            node.pointer("/defaultBranchRef/target/commit/oid")
                .and_then(|oid| oid.as_str())
        })
        .or_else(|| {
            node.pointer("/default_branch_ref/target/oid")
                .and_then(|oid| oid.as_str())
        });

    makeing_commit_hash_normal(raw_oid)
}

fn contains_zig(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(name) => name.eq_ignore_ascii_case("zig"),
        serde_json::Value::Array(items) => items.iter().any(contains_zig),
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(name)) = map.get("name") {
                if name.eq_ignore_ascii_case("zig") {
                    return true;
                }
            }
            if let Some(node) = map.get("node") {
                if contains_zig(node) {
                    return true;
                }
            }
            if let Some(edges) = map.get("edges") {
                if contains_zig(edges) {
                    return true;
                }
            }
            if let Some(nodes) = map.get("nodes") {
                if contains_zig(nodes) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

pub fn has_zig_in_top_languages(repository: &Node) -> bool {
    repository
        .languages
        .as_ref()
        .map(contains_zig)
        .unwrap_or(false)
}

fn has_zig_in_top_languages_value(repository: &serde_json::Value) -> bool {
    repository
        .get("languages")
        .map(contains_zig)
        .unwrap_or(false)
}

// pub async fn process_everything(
//     pool: Arc<Connection>,
//     query: String,
//     is_package: bool,
//     time_15_minutes_ago: NaiveDateTime,
// ) {
// }
// pub async fn process_last_15_minutes_part_1(
//     connection: Arc<Connection>,
//     query: String,
//     is_package: bool,
//     time_15_minutes_ago: NaiveDateTime,
// ) {
//     let client = Arc::new(create_optimized_client());
//     let mut has_next = true;
//     let mut next: Option<String> = None;

//     while has_next {
//         let query_to_send = serde_json::json!({
//             "query": GH_GRAPH_QL_QUERY,
//             "variables":  {
//                 "query": format!("topic:{query} stars:>20 pushed:>{}", time_15_minutes_ago.format("%Y-%m-%dT%H:%M:%SZ")),
//                 "next_value": next
//             }
//         });

//         let text = match fetch_with_retry(&client, query_to_send).await {
//             Some(text) => text,
//             None => {
//                 eprintln!(
//                     "STOPING THIS CYCLE! This process_last_15_minutes_part_1 cycle has failed.",
//                 );
//                 has_next = false;
//                 continue;
//             }
//         };

//         if text == EMPTY_REPLY {
//             has_next = false;
//             continue;
//         }

//         let res2: types::Root = match serde_json::from_str(&text) {
//             Ok(t) => t,
//             Err(t) => {
//                 eprintln!("Responce was in unexpected format process_last_15_minutes_part_1: {t}");
//                 has_next = false;
//                 continue;
//             }
//         };

//         has_next = res2.data.search.page_info.has_next_page;
//         next = res2.data.search.page_info.end_cursor;
//         let process_nodes = res2.data.search.nodes;

//         // I will process everything first, then commit to database.
//         let repo_data: Vec<RepoData> = stream::iter(process_nodes)
//             .filter(|node| futures::future::ready(has_zig_in_top_languages(node)))
//             .map(|node| {
//                 let client = Arc::clone(&client);
//                 async move { get_repo_data(&node, is_package, &client).await }
//             })
//             .buffer_unordered(25)
//             .collect()
//             .await;

//         let transaction = connection.transaction().await.unwrap();
//         for data in repo_data {
//             persist_repo_data(&transaction, data).await;
//         }
//         transaction.commit().await.unwrap();
//     }
// }

pub async fn get_repo_data(
    repository: &Node,
    is_package: bool,
    client: &reqwest::Client,
) -> RepoData {
    let user_id = format!("gh/{}", repository.owner.login).to_lowercase();
    let repo_id = format!("gh/{}/{}", repository.owner.login, repository.name).to_lowercase();

    let default_branch_name = repository
        .default_branch_ref
        .clone()
        .unwrap_or_default()
        .name
        .clone();

    let branch = if default_branch_name.is_empty() {
        "HEAD"
    } else {
        default_branch_name.as_ref()
    };

    let (build_zig_zon_data, (readme_url, readme_content)) = tokio::join!(
        get_build_zig_zon_data_wrapper(&repository.owner.login, &repository.name, branch, client),
        get_readme_url_and_content(
            &repository.owner.login,
            &repository.name,
            branch,
            true,
            client
        ),
    );

    let (readme_url, readme_content) = match (readme_url, readme_content) {
        (Some(url), Some(content)) => (url, content),
        (Some(url), None) => (url, String::new()),
        _ => ("404 unable to find readme. ".to_string(), String::new()),
    };

    let releases_iter = repository.releases.nodes.iter();
    let releases_futures = releases_iter.map(|release| {
        let owner = repository.owner.login.clone();
        let name = repository.name.clone();
        let tag = release.tag_name.clone();
        let release_clone = release.clone();

        async move {
            let ((readme_url, _), bzz_results) = tokio::join!(
                async {
                    match get_readme_url_and_content(&owner, &name, &tag, false, client).await {
                        (Some(url), _) => (url, String::new()),
                        _ => ("404 unable to find readme.".to_string(), String::new()),
                    }
                },
                get_build_zig_zon_data_wrapper(&owner, &name, &tag, client),
            );

            ReleaseData {
                tag_name: release_clone.tag_name,
                is_prerelease: release_clone.is_prerelease,
                published_at: release_clone.published_at,
                minimum_zig_version: bzz_results.0,
                readme_url,
                dependencies: bzz_results.1,
            }
        }
    });

    let releases = futures::future::join_all(releases_futures).await;
    let desc = repository.description.clone().unwrap_or(String::new());
    let readme_processed_content = crate::keyword_extraction(
        readme_content.as_str(),
        desc.as_str(),
        &repository.name.to_string(),
        &repository.owner.login.to_string(),
    )
    .await
    .unwrap();
    RepoData {
        repository: repository.clone(),
        is_package,
        user_id,
        repo_id,
        readme_url,
        readme_content: readme_processed_content,
        build_zig_zon_version: build_zig_zon_data.0,
        build_zig_zon_dependencies: build_zig_zon_data.1,
        releases,
    }
}

pub async fn persist_repo_data(transaction: &Transaction, data: RepoData) {
    let RepoData {
        repository,
        is_package,
        user_id,
        repo_id,
        readme_url,
        readme_content,
        build_zig_zon_version,
        build_zig_zon_dependencies,
        releases,
    } = data;

    let repo_id = truncate_to_char_limit(&repo_id, limits::REPO_ID_MAX_LEN);
    let user_id = truncate_to_char_limit(&user_id, limits::USER_ID_MAX_LEN);
    let platform_id = "gh";
    let avatar_id = truncate_to_char_limit(&repository.owner.login, limits::USER_AVATAR_ID_MAX_LEN);
    let owner_id = truncate_to_char_limit(&user_id, limits::REPO_OWNER_MAX_LEN);
    let user_bio =
        truncate_option_to_char_limit(repository.owner.bio.as_deref(), limits::USER_BIO_MAX_LEN)
            .or_else(|| {
                truncate_option_to_char_limit(
                    repository.owner.description.as_deref(),
                    limits::USER_BIO_MAX_LEN,
                )
            });
    let description = truncate_option_to_char_limit(
        repository.description.as_deref(),
        limits::REPO_DESCRIPTION_MAX_LEN,
    );
    let default_branch_ref = repository.default_branch_ref.as_ref();
    let default_branch_name = truncate_to_char_limit(
        default_branch_ref
            .map(|branch| branch.name.as_str())
            .unwrap_or_default(),
        limits::REPO_DEFAULT_BRANCH_MAX_LEN,
    );
    let latest_commit_hash = get_commit_hash(default_branch_ref);
    let license = truncate_to_char_limit(
        repository
            .license_info
            .as_ref()
            .map(|l| l.spdx_id.as_str())
            .unwrap_or("-"),
        limits::REPO_LICENSE_MAX_LEN,
    );
    let primary_language = truncate_to_char_limit(
        repository
            .primary_language
            .as_ref()
            .map(|lang| lang.name.as_str())
            .unwrap_or("-"),
        limits::REPO_PRIMARY_LANGUAGE_MAX_LEN,
    );
    let database_updated_at = chrono::Utc::now().timestamp();
    let pushed_at_epoch = parse_iso_to_epoch(&repository.pushed_at);
    let created_at_epoch = parse_iso_to_epoch(&repository.created_at);

    let latest_release = releases.iter().find(|r| !r.is_prerelease).or_else(|| releases.first());
    let latest_release_version = latest_release.map(|r| {
        truncate_to_char_limit(&r.tag_name, limits::RELEASE_VERSION_MAX_LEN)
    });
    let min_zig_ver_candidate = latest_release
        .map(|r| r.minimum_zig_version.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(build_zig_zon_version.as_str());
    let minimum_zig_version = if min_zig_ver_candidate.is_empty() {
        None
    } else {
        Some(truncate_to_char_limit(min_zig_ver_candidate, limits::RELEASE_MIN_ZIG_VERSION_MAX_LEN))
    };
    let owner_avatar_id = Some(avatar_id.clone());
    let is_package_flag = if is_package { 1i64 } else { 0i64 };
    let is_program_flag = if !is_package { 1i64 } else { 0i64 };

    let (user_insert_result, repo_insert_result) = tokio::join!(
        transaction.execute(
            r#"
            INSERT INTO users (id, platform_id, avatar_id, bio)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                platform_id = excluded.platform_id,
                avatar_id = excluded.avatar_id,
                bio = excluded.bio
            "#,
            params![user_id.clone(), platform_id, avatar_id.clone(), user_bio],
        ),
        transaction.execute(
            r#"
            INSERT INTO repos
                (id, owner, platform_id, description, issues_count, default_branch_name, fork_count,
                 stargazer_count, watchers_count, pushed_at, created_at, is_archived, is_disabled,
                 is_fork, license, primary_language, latest_commit_hash, last_updated_in_this_database,
                 is_package, is_program, latest_release_version, dependents_count, owner_avatar_id, minimum_zig_version)
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                owner = excluded.owner,
                platform_id = excluded.platform_id,
                description = excluded.description,
                issues_count = excluded.issues_count,
                default_branch_name = excluded.default_branch_name,
                fork_count = excluded.fork_count,
                stargazer_count = excluded.stargazer_count,
                watchers_count = excluded.watchers_count,
                pushed_at = excluded.pushed_at,
                created_at = excluded.created_at,
                is_archived = excluded.is_archived,
                is_disabled = excluded.is_disabled,
                is_fork = excluded.is_fork,
                license = excluded.license,
                primary_language = excluded.primary_language,
                latest_commit_hash = excluded.latest_commit_hash,
                last_updated_in_this_database = excluded.last_updated_in_this_database,
                is_package = CASE WHEN excluded.is_package = 1 THEN 1 ELSE repos.is_package END,
                is_program = CASE WHEN excluded.is_program = 1 THEN 1 ELSE repos.is_program END,
                latest_release_version = COALESCE(excluded.latest_release_version, repos.latest_release_version),
                dependents_count = repos.dependents_count,
                owner_avatar_id = COALESCE(excluded.owner_avatar_id, repos.owner_avatar_id),
                minimum_zig_version = COALESCE(excluded.minimum_zig_version, repos.minimum_zig_version)
            "#,
            params![
                repo_id.clone(),
                owner_id,
                platform_id,
                description,
                repository.issues.total_count,
                default_branch_name,
                repository.fork_count,
                repository.stargazer_count,
                repository.watchers.total_count,
                pushed_at_epoch,
                created_at_epoch,
                repository.is_archived,
                repository.is_disabled,
                repository.is_fork,
                license,
                primary_language,
                latest_commit_hash,
                database_updated_at,
                is_package_flag,
                is_program_flag,
                latest_release_version,
                0i64,
                owner_avatar_id,
                minimum_zig_version,
            ],
        ),
    );
    user_insert_result.unwrap();
    repo_insert_result.unwrap();
    transaction
        .execute(
            r#"DELETE FROM repo_search WHERE repo_id = ?"#,
            params![repo_id.clone()],
        )
        .await
        .unwrap();
    transaction
        .execute(
            r#"INSERT INTO repo_search (repo_id, keywords) VALUES (?, ?)"#,
            params![repo_id.clone(), readme_content],
        )
        .await
        .unwrap();

    transaction
        .execute(
            "DELETE FROM repo_topics WHERE repo_id = ?",
            params![repo_id.clone()],
        )
        .await
        .unwrap();

    let mut topics: Vec<String> = repository
        .repository_topics
        .nodes
        .iter()
        .map(|element| truncate_to_char_limit(&element.topic.name, limits::TOPIC_MAX_LEN))
        .filter(|topic| !topic.is_empty())
        .collect();
    topics.extend(
        repository
            .repository_topics
            .edges
            .iter()
            .map(|element| truncate_to_char_limit(&element.node.topic.name, limits::TOPIC_MAX_LEN))
            .filter(|topic| !topic.is_empty()),
    );

    let mut seen = std::collections::HashSet::new();
    topics.retain(|topic| seen.insert(topic.clone()));

    for topic in topics {
        transaction
            .execute(
                r#"INSERT OR IGNORE INTO repo_topics (repo_id, topic) VALUES (?, ?)"#,
                params![repo_id.clone(), topic],
            )
            .await
            .unwrap();
    }

    let default_branch_version = truncate_to_char_limit(
        "__ZIGISTRY__DEFAULT__BRANCH__",
        limits::RELEASE_VERSION_MAX_LEN,
    );

    transaction
        .execute(
            r#"
            INSERT INTO releases
                (repo_id, version, is_prerelease, published_at, minimum_zig_version, readme_url)
            VALUES(?, ?, ?, ?, ?, ?)
            ON CONFLICT(repo_id, version) DO UPDATE SET
                is_prerelease = excluded.is_prerelease,
                published_at = excluded.published_at,
                minimum_zig_version = excluded.minimum_zig_version,
                readme_url = excluded.readme_url
            "#,
            params![
                repo_id.clone(),
                default_branch_version.clone(),
                false,
                created_at_epoch,
                truncate_option_to_char_limit(
                    if build_zig_zon_version.is_empty() {
                        None
                    } else {
                        Some(&build_zig_zon_version)
                    },
                    limits::RELEASE_MIN_ZIG_VERSION_MAX_LEN,
                ),
                readme_url.clone(),
            ],
        )
        .await
        .unwrap();

    transaction
        .execute(
            "DELETE FROM release_dependencies WHERE repo_id = ? AND version = ?",
            params![repo_id.clone(), default_branch_version.clone()],
        )
        .await
        .unwrap();

    if !build_zig_zon_dependencies.is_empty() {
        let placeholders = build_zig_zon_dependencies
            .iter()
            .map(|_| "(?, ?, ?, ?, ?, ?, ?)")
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "INSERT INTO release_dependencies (repo_id, version, name, hash, is_lazy, url, path) VALUES {}",
            placeholders
        );

        let mut params_vec: Vec<libsql::Value> = Vec::new();
        for dependency in &build_zig_zon_dependencies {
            params_vec.push(repo_id.clone().into());
            params_vec.push(default_branch_version.clone().into());
            params_vec.push(
                truncate_to_char_limit(&dependency.name, limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN)
                    .into(),
            );
            params_vec.push(
                truncate_to_char_limit(&dependency.hash, limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN)
                    .into(),
            );
            params_vec.push(i64::from(parse_lazy_flag(&dependency.lazy)).into());
            params_vec.push(
                truncate_to_char_limit(&dependency.url, limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN)
                    .into(),
            );
            params_vec.push(
                truncate_to_char_limit(&dependency.path, limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN)
                    .into(),
            );
        }

        transaction.execute(&sql, params_vec).await.unwrap();
    }

    for release_data in releases {
        let release_version =
            truncate_to_char_limit(&release_data.tag_name, limits::RELEASE_VERSION_MAX_LEN);
        let release_published_at_epoch = parse_iso_to_epoch(&release_data.published_at);

        transaction
            .execute(
                r#"
                INSERT INTO releases
                    (repo_id, version, is_prerelease, published_at, minimum_zig_version, readme_url)
                VALUES(?, ?, ?, ?, ?, ?)
                ON CONFLICT(repo_id, version) DO UPDATE SET
                    is_prerelease = excluded.is_prerelease,
                    published_at = excluded.published_at,
                    minimum_zig_version = excluded.minimum_zig_version,
                    readme_url = excluded.readme_url
                "#,
                params![
                    repo_id.clone(),
                    release_version.clone(),
                    release_data.is_prerelease,
                    release_published_at_epoch,
                    truncate_option_to_char_limit(
                        if release_data.minimum_zig_version.is_empty() {
                            None
                        } else {
                            Some(&release_data.minimum_zig_version)
                        },
                        limits::RELEASE_MIN_ZIG_VERSION_MAX_LEN,
                    ),
                    release_data.readme_url,
                ],
            )
            .await
            .unwrap();

        transaction
            .execute(
                "DELETE FROM release_dependencies WHERE repo_id = ? AND version = ?",
                params![repo_id.clone(), release_version.clone()],
            )
            .await
            .unwrap();

        if !release_data.dependencies.is_empty() {
            let placeholders = release_data
                .dependencies
                .iter()
                .map(|_| "(?, ?, ?, ?, ?, ?, ?)")
                .collect::<Vec<_>>()
                .join(", ");

            let sql = format!(
                "INSERT INTO release_dependencies (repo_id, version, name, hash, is_lazy, url, path) VALUES {}",
                placeholders
            );

            let mut params_vec: Vec<libsql::Value> = Vec::new();
            for dependency in &release_data.dependencies {
                params_vec.push(repo_id.clone().into());
                params_vec.push(release_version.clone().into());
                params_vec.push(
                    truncate_to_char_limit(
                        &dependency.name,
                        limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN,
                    )
                    .into(),
                );
                params_vec.push(
                    truncate_to_char_limit(
                        &dependency.hash,
                        limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN,
                    )
                    .into(),
                );
                params_vec.push(i64::from(parse_lazy_flag(&dependency.lazy)).into());
                params_vec.push(
                    truncate_to_char_limit(
                        &dependency.url,
                        limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN,
                    )
                    .into(),
                );
                params_vec.push(
                    truncate_to_char_limit(
                        &dependency.path,
                        limits::RELEASE_DEPENDENCY_FIELD_MAX_LEN,
                    )
                    .into(),
                );
            }

            transaction.execute(&sql, params_vec).await.unwrap();
        }
    }
}

pub async fn process_query_range_complete_repo_query(
    query: &str,
    is_package: bool,
    pool: Arc<Connection>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    step: u64,
    stars_filter: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lower = start_date;
    let base_step = step.max(1); // Again, I am doing this to make sure the minimum step is 1 day.
    let mut dynamic_step = base_step;
    let client = Arc::new(create_optimized_client());

    loop {
        if lower > end_date {
            break;
        }

        let upper = lower.checked_add_days(Days::new(dynamic_step)).unwrap();
        let mut nodes = Vec::new();
        eprintln!("Now processing:{lower}..{upper}");
        let mut has_next = true;
        let mut next: Option<String> = None;
        let mut window_failed = false;

        while has_next {
            let query_to_send = serde_json::json!({
                "query": GH_GRAPH_QL_QUERY,
                "variables":  {
                    // I know this could be done in a single function.
                    // but specifically for testing, I am doing this.
                    "query": format!("topic:{query} {stars_filter} created:{}..{}", lower.format("%Y-%m-%dT%H:%M:%SZ"), upper.format("%Y-%m-%dT%H:%M:%SZ")),
                    "next_value": next
                }
            });

            let text = match fetch_with_retry(&client, query_to_send).await {
                Some(text) => text,
                None => {
                    eprintln!("Window failed for: {lower}..{upper}. step was {dynamic_step}");
                    window_failed = true;
                    break;
                }
            };

            if text == EMPTY_REPLY {
                has_next = false;
                continue;
            }

            let mut res2: types::Root = match serde_json::from_str(&text) {
                Ok(t) => t,
                Err(t) => {
                    eprintln!("Got this response:  {text}");
                    eprintln!("parsing failed: {t}");
                    has_next = false;
                    continue;
                }
            };

            eprintln!("{:#?}", res2.data.search.page_info.has_next_page);
            has_next = res2.data.search.page_info.has_next_page;
            next = res2.data.search.page_info.end_cursor;
            nodes.append(&mut res2.data.search.nodes);
        }

        if window_failed {
            if dynamic_step > 1 {
                let new_step = (dynamic_step / 2).max(1); // Maybe window is too big, hence, also I am doing max (1) to make sure, the minimum step is 1.
                eprintln!("Reducing {dynamic_step} to {new_step}");
                dynamic_step = new_step;
            } else {
                eprintln!("WARNING! SKIPPING {lower}..{upper}.");
                lower = upper;
                dynamic_step = base_step;
            }
            continue;
        }

        let repo_data: Vec<RepoData> = stream::iter(nodes)
            .filter(|node| futures::future::ready(has_zig_in_top_languages(node)))
            .map(|node| {
                let client = Arc::clone(&client);
                println!("Processing repo: {}/{}", node.owner.login, node.name);
                async move { get_repo_data(&node, is_package, &client).await }
            })
            .buffer_unordered(ASYNC_LIMIT)
            .collect()
            .await;

        // Now do all database operations in a quick transaction
        let transaction = pool.transaction().await.unwrap();
        for data in repo_data {
            persist_repo_data(&transaction, data).await;
        }
        transaction.commit().await.unwrap();

        lower = upper;
        dynamic_step = base_step;
    }
    Ok(())
}

pub async fn github_main(
    pool: Arc<Connection>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    step: u64,
    stars_filter: &str,
) -> Result<(), Box<dyn Error>> {
    process_query_range_complete_repo_query(
        "zig-package",
        true,
        Arc::clone(&pool),
        start_date,
        end_date,
        step,
        stars_filter,
    )
    .await?;
    process_query_range_complete_repo_query(
        "zig",
        false,
        pool,
        start_date,
        end_date,
        step,
        stars_filter,
    )
    .await?;
    Ok(())
}

#[derive(Debug)]
struct PartialRepoNode {
    owner_login: String,
    owner_id: String,
    repo_id: String,
    latest_commit_hash: String,
}

fn parse_partial_repo_node(node: &serde_json::Value) -> Option<PartialRepoNode> {
    let (owner_login, repo_name) = if let Some(name_with_owner) = node
        .get("nameWithOwner")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let mut split = name_with_owner.splitn(2, '/');
        let owner_login = split.next()?.trim().to_lowercase();
        let repo_name = split.next()?.trim().to_lowercase();
        (owner_login, repo_name)
    } else {
        let owner_login = node
            .get("owner")
            .and_then(|owner| owner.get("login"))
            .and_then(|value| value.as_str())
            .map(str::trim)?
            .to_lowercase();

        let repo_name = node
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::trim)?
            .to_lowercase();

        (owner_login, repo_name)
    };

    if owner_login.is_empty() || repo_name.is_empty() {
        return None;
    }

    let latest_commit_hash = get_commit_hash_value(node);

    Some(PartialRepoNode {
        owner_id: format!("gh/{owner_login}"),
        repo_id: format!("gh/{owner_login}/{repo_name}"),
        owner_login,
        latest_commit_hash,
    })
}

pub async fn process_query_range_partial_repo_query(
    query: &str,
    is_package: bool,
    pool: Arc<Connection>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    step: u64,
    stars_filter: &str,
) -> Result<(), Box<dyn Error>> {
    let mut lower = start_date;
    let base_step = step.max(1); // Again, I am doing this to make sure the minimum step is 1 day.
    let mut dynamic_step = base_step;
    let client = Arc::new(create_optimized_client());

    loop {
        if lower > end_date {
            break;
        }

        let upper = lower.checked_add_days(Days::new(dynamic_step)).unwrap();
        let mut nodes = Vec::new();
        eprintln!("Now processing:{lower}..{upper}");
        let mut has_next = true;
        let mut next: Option<String> = None;
        let mut window_failed = false;

        while has_next {
            let query_to_send = serde_json::json!({
                "query": GH_GRAPH_QL_PARTIAL_QUERY,
                "variables":  {
                    "query": format!("topic:{query} {stars_filter} created:{}..{}", lower.format("%Y-%m-%dT%H:%M:%SZ"), upper.format("%Y-%m-%dT%H:%M:%SZ")),
                    "next_value": next
                }
            });

            let text = match fetch_with_retry(&client, query_to_send).await {
                Some(text) => text,
                None => {
                    eprintln!("Window failed for: {lower}..{upper}. step was {dynamic_step}");
                    window_failed = true;
                    break;
                }
            };

            if text == EMPTY_REPLY {
                has_next = false;
                continue;
            }

            let response_json: serde_json::Value = match serde_json::from_str(&text) {
                Ok(t) => t,
                Err(t) => {
                    eprintln!("Got this response:  {text}");
                    eprintln!("parsing failed: {t}");
                    has_next = false;
                    continue;
                }
            };

            has_next = response_json
                .get("data")
                .and_then(|data| data.get("search"))
                .and_then(|search| search.get("pageInfo"))
                .and_then(|page| page.get("hasNextPage"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            next = response_json
                .get("data")
                .and_then(|data| data.get("search"))
                .and_then(|search| search.get("pageInfo"))
                .and_then(|page| page.get("endCursor"))
                .and_then(|value| value.as_str())
                .map(str::to_string);

            if let Some(page_nodes) = response_json
                .get("data")
                .and_then(|data| data.get("search"))
                .and_then(|search| search.get("nodes"))
                .and_then(|value| value.as_array())
            {
                for node in page_nodes {
                    nodes.push(node.clone());
                }
            }
        }

        if window_failed {
            if dynamic_step > 1 {
                let new_step = (dynamic_step / 2).max(1); // Maybe window is too big, hence, also I am doing max (1) to make sure, the minimum step is 1.
                eprintln!("Reducing {dynamic_step} to {new_step}");
                dynamic_step = new_step;
            } else {
                eprintln!("WARNING! SKIPPING {lower}..{upper}.");
                lower = upper;
                dynamic_step = base_step;
            }
            continue;
        }

        let repo_type = if is_package { "package" } else { "program" };
        let transaction = pool.transaction().await?;
        for node in nodes {
            if !has_zig_in_top_languages_value(&node) {
                continue;
            }

            let parsed = match parse_partial_repo_node(&node) {
                Some(parsed) => parsed,
                None => continue,
            };

            let mut existing_rows = transaction
                .query(
                    "SELECT latest_commit_hash FROM repos WHERE id = ? LIMIT 1",
                    params![parsed.repo_id.clone()],
                )
                .await?;

            if let Some(row) = existing_rows.next().await? {
                let existing_commit_hash: String = row.get(0)?;
                if existing_commit_hash != parsed.latest_commit_hash {
                    let now_epoch = chrono::Utc::now().timestamp();
                    transaction
                        .execute(
                            r#"
                            INSERT INTO repo_pipeline_queue (id, type_of_repo, status, queued_at)
                            VALUES (?, ?, 'needs_update', ?)
                            ON CONFLICT(id) DO UPDATE SET
                                status = 'needs_update',
                                queued_at = excluded.queued_at
                            "#,
                            params![parsed.repo_id, repo_type, now_epoch],
                        )
                        .await?;
                }
                continue;
            }

            let mut banned_rows = transaction
                .query(
                    "SELECT 1 FROM banned_users WHERE id IN (?, ?) LIMIT 1",
                    params![parsed.owner_id, parsed.owner_login],
                )
                .await?;

            if banned_rows.next().await?.is_some() {
                continue;
            }

            let now_epoch = chrono::Utc::now().timestamp();
            transaction
                .execute(
                    r#"
                    INSERT INTO repo_pipeline_queue (id, type_of_repo, status, queued_at)
                    VALUES (?, ?, 'pending_check', ?)
                    ON CONFLICT(id) DO NOTHING
                    "#,
                    params![parsed.repo_id, repo_type, now_epoch],
                )
                .await?;
        }
        transaction.commit().await?;

        lower = upper;
        dynamic_step = base_step;
    }
    Ok(())
}

pub async fn github_main_cron(
    pool: Arc<Connection>,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    step: u64,
    stars_filter: &str,
) -> Result<(), Box<dyn Error>> {
    process_query_range_partial_repo_query(
        "zig-package",
        true,
        Arc::clone(&pool),
        start_date,
        end_date,
        step,
        stars_filter,
    )
    .await?;
    process_query_range_partial_repo_query(
        "zig",
        false,
        pool,
        start_date,
        end_date,
        step,
        stars_filter,
    )
    .await?;
    Ok(())
}

pub async fn get_readme_url_and_content(
    owner_name: &str,
    repo_name: &str,
    branch_or_tag: &str,
    fetch_content: bool,
    client: &reqwest::Client,
) -> (Option<String>, Option<String>) {
    let url =
        format!("https://api.github.com/repos/{owner_name}/{repo_name}/readme?ref={branch_or_tag}");

    let resp = match client
        .get(&url)
        .header("Authorization", GITHUB_KEY.to_string())
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "zigistry")
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp,
        _ => return (None, None),
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let download_url = match body["download_url"].as_str() {
        Some(u) => u.to_string(),
        None => return (None, None),
    };

    if fetch_content {
        let content = match client
            .get(&download_url)
            .header("User-Agent", "zigistry")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            _ => return (Some(download_url), None),
        };

        (Some(download_url), Some(content))
    } else {
        (Some(download_url), None)
    }
}

// now, I am checking a head request, and then doing get.
pub async fn get_build_zig_zon_data(
    owner_name: &str,
    repo_name: &str,
    branch_or_tag: &str,
    client: &reqwest::Client,
) -> Result<(String, Vec<custom_types::Dependency>), Box<dyn Error>> {
    let url = format!(
        "https://raw.githubusercontent.com/{owner_name}/{repo_name}/{branch_or_tag}/build.zig.zon"
    );

    let head_response = client.head(&url).send().await?;
    if !head_response.status().is_success() {
        return Err("build.zig.zon not found".into());
    }

    let text = client.get(&url).send().await?.text().await?;

    let tokens = tokenize(text.chars())?;
    let parsed = parse(tokens.into_iter())?;

    Ok((parsed.minimum_zig_version, parsed.dependencies))
}

async fn get_build_zig_zon_data_wrapper(
    owner_name: &str,
    repo_name: &str,
    branch_or_tag: &str,
    client: &reqwest::Client,
) -> (String, Vec<custom_types::Dependency>) {
    match get_build_zig_zon_data(owner_name, repo_name, branch_or_tag, client).await {
        Ok((minimum_zig_version, dependencies)) => (minimum_zig_version, dependencies),
        Err(_) => ("unknown".to_string(), Vec::new()),
    }
}


/// I have added this new client, which is much more optimized
/// becuase, initially, I wasn't adding any timeouts.
fn create_optimized_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(90)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

const MAX_RETRIES: usize = 4;
const INITIAL_WAITING_TIME: u64 = 2;
const MAX_WAITING_TIME: u64 = 16;

// Helper to reduce code duplication
async fn fetch_with_retry(
    client: &reqwest::Client,
    query_to_send: serde_json::Value,
) -> Option<String> {
    let mut retry_count = 0usize;
    let mut waiting_time = INITIAL_WAITING_TIME;
    let mut client = client.clone(); // allow replacement

    loop {
        if retry_count >= MAX_RETRIES {
            eprintln!("Tried {retry_count} times, still problem. Returning failure.");
            return None;
        }

        match client
            .post("https://api.github.com/graphql")
            .header("Authorization", GITHUB_KEY.to_string())
            .header("User-Agent", "zigistry.dev")
            .json(&query_to_send)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    eprintln!("GitHub API error: {status}");
                    retry_count += 1;

                    if status == 502 {
                        eprintln!("502 Bad Gateway - recreating client");
                        client = create_optimized_client();
                    }

                    if status == 429 || status == 403 || status == 502 {
                        waiting_time = (waiting_time * 2).min(MAX_WAITING_TIME);
                        eprintln!("Backing off for {waiting_time}s before retry");
                    }

                    tokio::time::sleep(Duration::from_secs(waiting_time)).await;
                    continue;
                }

                match resp.text().await {
                    Ok(body) if body.is_empty() => {
                        eprintln!(
                            "GitHub returned empty body (attempt {retry_count}), retrying..."
                        );
                        retry_count += 1;
                        tokio::time::sleep(Duration::from_secs(waiting_time)).await;
                        continue;
                    }
                    Ok(body) => return Some(body),
                    Err(e) => {
                        eprintln!("problem: {e}");
                        retry_count += 1;
                        waiting_time = (waiting_time * 2).min(MAX_WAITING_TIME);
                        tokio::time::sleep(Duration::from_secs(waiting_time)).await;
                        continue;
                    }
                }
            }
            Err(e) => {
                eprintln!("GitHub Error: {e}");
                retry_count += 1;
                waiting_time = (waiting_time * 2).min(MAX_WAITING_TIME);
                tokio::time::sleep(Duration::from_secs(waiting_time.max(10))).await;
                continue;
            }
        }
    }
}

#[tokio::test]
async fn test() {
    let client = reqwest::Client::new();
    let res = get_readme_url_and_content("zigistry", "zigistry", "main", true, &client).await;
    let url = res.0.unwrap();

    let content = res.1.unwrap();

    println!("content: {content}");
    println!("url: {url}");
}
