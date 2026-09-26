mod codeberg_data;
mod codeberg_process_release;
pub mod helper_functions;
pub mod types;
use std::sync::Arc;

use crate::codeberg::codeberg_data::RepoData;
use crate::codeberg::helper_functions::{
    fetch_root_folder_directory_files, get_build_zig_zon_data, get_latest_commit_hash,
    has_zig_in_top_languages,
};
use crate::codeberg::types::types::Daum;
use crate::constants::ASYNC_LIMIT;
use crate::constants::limits;
use crate::database::{parse_lazy_flag, truncate_option_to_char_limit, truncate_to_char_limit};
use crate::{CODEBERG_KEY, codeberg::helper_functions::get_readme_url};
use codeberg_process_release::fetch_releases;
use futures::{stream, stream::StreamExt};
use libsql::{Connection, Transaction, params};

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

pub async fn get_repo_data(repository: Daum) -> RepoData {
    let user_id = format!("cb/{}", repository.owner.login).to_lowercase();
    let repo_id = format!("cb/{}/{}", repository.owner.login, repository.name).to_lowercase();
    let latest_commit_hash =
        get_latest_commit_hash(&repository.owner.login, &repository.name).await;
    let client = reqwest::Client::new();
    let default_branch_directory_files = fetch_root_folder_directory_files(
        &client,
        &repository.owner.login,
        &repository.name,
        &repository.default_branch,
    )
    .await;

    let (readme_url, readme_content) = get_readme_url(
        &repository.owner.login,
        repository.name.as_str(),
        &repository.default_branch,
        false,
        true,
        &default_branch_directory_files,
    )
    .await;

    let build_zig_zon_data = match get_build_zig_zon_data(
        &repository.owner.login,
        &repository.name,
        "HEAD",
        false,
    )
    .await
    {
        Ok(t) => t,
        Err(_) => (String::new(), Vec::new()),
    };

    let releases = fetch_releases(&repository.owner.login, &repository.name).await;
    let desc = repository.description.clone();
    let readme_processed_content = crate::keyword_extraction(
        readme_content.as_str(),
        desc.as_str(),
        &repository.name.to_string(),
        &repository.owner.login.to_string(),
    )
    .await
    .unwrap();
    RepoData {
        repository,
        user_id,
        repo_id,
        latest_commit_hash,
        readme_url,
        readme_content: readme_processed_content,
        build_zig_zon_version: build_zig_zon_data.0,
        build_zig_zon_dependencies: build_zig_zon_data.1,
        default_branch_directory_files,
        releases,
    }
}

pub async fn send_repo_data_to_database(transaction: &Transaction, data: RepoData) {
    let RepoData {
        repository,
        user_id,
        repo_id,
        latest_commit_hash,
        readme_url,
        readme_content,
        build_zig_zon_version,
        build_zig_zon_dependencies,
        default_branch_directory_files: _,
        releases,
    } = data;

    let repo_id = truncate_to_char_limit(&repo_id, limits::REPO_ID_MAX_LEN);
    let user_id = truncate_to_char_limit(&user_id, limits::USER_ID_MAX_LEN);
    let platform_id = "cb";
    let avatar_id = truncate_to_char_limit(
        repository
            .owner
            .avatar_url
            .rsplit('/')
            .next()
            .unwrap_or(repository.owner.login.as_str()),
        limits::USER_AVATAR_ID_MAX_LEN,
    );
    let owner_id = truncate_to_char_limit(&user_id, limits::REPO_OWNER_MAX_LEN);
    let user_bio = Some(truncate_to_char_limit(
        &repository.owner.description,
        limits::USER_BIO_MAX_LEN,
    ));
    let description = Some(truncate_to_char_limit(
        &repository.description,
        limits::REPO_DESCRIPTION_MAX_LEN,
    ));
    let default_branch_name = truncate_to_char_limit(
        &repository.default_branch,
        limits::REPO_DEFAULT_BRANCH_MAX_LEN,
    );
    let latest_commit_hash =
        truncate_to_char_limit(&latest_commit_hash, limits::REPO_COMMIT_HASH_MAX_LEN);
    let license = truncate_to_char_limit("-", limits::REPO_LICENSE_MAX_LEN);
    let primary_language =
        truncate_to_char_limit(&repository.language, limits::REPO_PRIMARY_LANGUAGE_MAX_LEN);
    let database_updated_at = chrono::Utc::now().timestamp();
    let pushed_at_epoch = parse_iso_to_epoch(&repository.updated_at);
    let created_at_epoch = parse_iso_to_epoch(&repository.created_at);
    let is_package = repository.topics.iter().any(|topic| topic == "zig-package");

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

    let (user_insert_result, repo_insert_result) = tokio::join!(
        transaction.execute(
            r#"
            INSERT INTO users
                (id, platform_id, avatar_id, bio)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                platform_id = excluded.platform_id,
                avatar_id = excluded.avatar_id,
                bio = excluded.bio
            "#,
            params![user_id.clone(), platform_id, avatar_id, user_bio],
        ),
        transaction.execute(
            r#"
            INSERT INTO repos
                (id, owner, platform_id, description, issues_count, default_branch_name, fork_count,
                 stargazer_count, watchers_count, pushed_at, created_at, is_archived, is_disabled,
                 is_fork, license, primary_language, latest_commit_hash, last_updated_in_this_database,
                 is_package, is_program, latest_release_version, dependents_count, owner_avatar_id, minimum_zig_version)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                is_package = (excluded.is_package OR repos.is_package),
                is_program = (excluded.is_program OR repos.is_program),
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
                repository.open_issues_count,
                default_branch_name,
                repository.forks_count,
                repository.stars_count,
                repository.watchers_count,
                pushed_at_epoch,
                created_at_epoch,
                repository.archived,
                repository.archived,
                repository.fork,
                license,
                primary_language,
                latest_commit_hash,
                database_updated_at,
                is_package,
                !is_package,
                latest_release_version,
                0i64,
                owner_avatar_id,
                minimum_zig_version,
            ]
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
        .topics
        .iter()
        .map(|topic| truncate_to_char_limit(topic, limits::TOPIC_MAX_LEN))
        .filter(|topic| !topic.is_empty())
        .collect();

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
                readme_url,
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

    for r in releases {
        let release_version =
            truncate_to_char_limit(&r.tag_name, limits::RELEASE_VERSION_MAX_LEN);
        let release_published_at_epoch = parse_iso_to_epoch(&r.published_at);

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
                    r.is_prerelease,
                    release_published_at_epoch,
                    truncate_option_to_char_limit(
                        if r.minimum_zig_version.is_empty() {
                            None
                        } else {
                            Some(&r.minimum_zig_version)
                        },
                        limits::RELEASE_MIN_ZIG_VERSION_MAX_LEN,
                    ),
                    r.readme_url,
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

        if !r.dependencies.is_empty() {
            let placeholders = r
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
            for dependency in &r.dependencies {
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

pub async fn fetch_all_codeberg_repos(
    pool: Arc<Connection>,
    query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut page = 1;
    let client = reqwest::Client::new();
    loop {
        let url = format!(
            "https://codeberg.org/api/v1/repos/search?q={query}&limit=100&page={page}&topic=true",
        );

        eprintln!("Processing: {}", url);

        let mut responce = Option::None;
        for attempt_count in 0..5 {
            match client
                .get(&url)
                .header("Authorization", &*CODEBERG_KEY)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        eprintln!("Codeberg status: {}", resp.status());
                        let wait_secs = if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                            resp.headers()
                                .get("Retry-After")
                                .and_then(|v| v.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(60)
                        } else {
                            2u64.pow(attempt_count)
                        };
                        eprintln!("Waiting {} seconds before retry...", wait_secs);
                        tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                        continue;
                    }

                    match resp.text().await {
                        Ok(body) => match serde_json::from_str::<types::types::Root>(&body) {
                            Ok(val) => {
                                responce = Some(val);
                                break;
                            }
                            Err(e) => {
                                let snippet: String = body.chars().take(300).collect();
                                eprintln!("Failed to parse JSON: {}", e);
                                eprintln!("Codeberg body (truncated): {}", snippet);
                                tokio::time::sleep(std::time::Duration::from_secs(
                                    2u64.pow(attempt_count),
                                ))
                                .await;
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read response body: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(
                                2u64.pow(attempt_count),
                            ))
                            .await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to send request: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt_count)))
                        .await;
                }
            }
        }

        let responce = responce.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch data from Codeberg after 5 retries",
            )
        })?;

        if responce.data.is_empty() {
            break;
        }

        let transaction = pool.transaction().await.unwrap();
        stream::iter(responce.data)
            .map(|repository| async move {
                if !has_zig_in_top_languages(&repository.owner.login, &repository.name).await {
                    return None;
                }
                Some(get_repo_data(repository).await)
            })
            .buffer_unordered(ASYNC_LIMIT)
            .for_each(|data| async {
                if let Some(data) = data {
                    send_repo_data_to_database(&transaction, data).await;
                }
            })
            .await;

        transaction.commit().await.unwrap();
        page += 1;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    Ok(())
}

pub async fn codeberg_main(pool: Arc<Connection>) -> Result<(), Box<dyn std::error::Error>> {
    fetch_all_codeberg_repos(pool.clone(), "zig-package")
        .await
        .unwrap();
    fetch_all_codeberg_repos(pool.clone(), "zig").await.unwrap();
    Ok(())
}

pub async fn fetch_all_codeberg_repos_cron_updating_part(
    pool: Arc<Connection>,
    query: &str,
    is_package: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut page = 1;
    let client = reqwest::Client::new();
    loop {
        let url = format!(
            "https://codeberg.org/api/v1/repos/search?q={query}&limit=100&page={page}&topic=true",
        );

        eprintln!("Processing cron: {}", url);

        let mut responce = Option::None;
        for number_of_tries_done in 0..5 {
            match client
                .get(&url)
                .header("Authorization", &*crate::CODEBERG_KEY)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        eprintln!("cb status: {}", resp.status());
                        let wait_secs = if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                            resp.headers()
                                .get("Retry-After")
                                .and_then(|v| v.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(60)
                        } else {
                            2u64.pow(number_of_tries_done)
                        };
                        eprintln!("Waiting {} seconds before trying again...", wait_secs);
                        tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                        continue;
                    }

                    match resp.text().await {
                        Ok(body) => match serde_json::from_str::<types::types::Root>(&body) {
                            Ok(val) => {
                                responce = Some(val);
                                break;
                            }
                            Err(e) => {
                                let snippet: String = body.chars().take(300).collect();
                                eprintln!("failed to parse json: {}", e);
                                eprintln!("cb body small: {}", snippet);
                                tokio::time::sleep(std::time::Duration::from_secs(
                                    2u64.pow(number_of_tries_done),
                                ))
                                .await;
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read response body: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(
                                2u64.pow(number_of_tries_done),
                            ))
                            .await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to send request: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(
                        2u64.pow(number_of_tries_done),
                    ))
                    .await;
                }
            }
        }

        let responce = responce.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch data from cb after 5 retries",
            )
        })?;

        if responce.data.is_empty() {
            break;
        }

        let repo_type = if is_package { "package" } else { "program" };
        let transaction = loop {
            match pool.transaction().await {
                Ok(t) => break t,
                Err(e) => {
                    eprintln!("transaction error, trying again. {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        };

        stream::iter(responce.data)
            .map(|repository| async move {
                if !crate::codeberg::helper_functions::has_zig_in_top_languages(&repository.owner.login, &repository.name).await {
                    return None;
                }

                let repo_id = format!("cb/{}/{}", repository.owner.login, repository.name).to_lowercase();
                let latest_commit_hash = crate::codeberg::helper_functions::get_latest_commit_hash(&repository.owner.login, &repository.name).await;
                Some((repository, repo_id, latest_commit_hash))
            })
            .buffer_unordered(crate::constants::ASYNC_LIMIT)
            .for_each(|data| async {
                if let Some((repository, repo_id, latest_commit_hash)) = data {
                    let existing_rows = transaction
                        .query(
                            "SELECT latest_commit_hash FROM repos WHERE id = ? LIMIT 1",
                            params![repo_id.clone()],
                        )
                        .await;

                    let mut existing_rows = match existing_rows {
                        Ok(rows) => rows,
                        Err(e) => {
                            eprintln!("db problem: {}", e);
                            return;
                        }
                    };

                    if let Ok(Some(row)) = existing_rows.next().await {
                        if let Ok(existing_commit_hash) = row.get::<String>(0) {
                            if existing_commit_hash != latest_commit_hash {
                                let now_epoch = chrono::Utc::now().timestamp();
                                if let Err(e) = transaction
                                    .execute(
                                        r#"
                                        INSERT INTO repo_pipeline_queue (id, type_of_repo, status, queued_at)
                                        VALUES (?, ?, 'needs_update', ?)
                                        ON CONFLICT(id) DO UPDATE SET
                                            status = 'needs_update',
                                            queued_at = excluded.queued_at
                                        "#,
                                        params![repo_id.clone(), repo_type, now_epoch],
                                    )
                                    .await
                                {
                                    eprintln!("db problem: {}", e);
                                }
                            }
                        }
                        return;
                    }

                    let banned_repos = transaction
                        .query(
                            "SELECT 1 FROM banned_users WHERE id IN (?, ?) LIMIT 1",
                            params![format!("cb/{}", repository.owner.login).to_lowercase(), repository.owner.login.to_lowercase()],
                        )
                        .await;

                    let mut banned_repos = match banned_repos {
                        Ok(rows) => rows,
                        Err(e) => {
                            eprintln!("db problem: {}", e);
                            return;
                        }
                    };

                    if let Ok(Some(_)) = banned_repos.next().await {
                        return;
                    }

                    let now_epoch = chrono::Utc::now().timestamp();
                    if let Err(e) = transaction
                        .execute(
                            r#"
                            INSERT INTO repo_pipeline_queue (id, type_of_repo, status, queued_at)
                            VALUES (?, ?, 'pending_check', ?)
                            ON CONFLICT(id) DO NOTHING
                            "#,
                            params![repo_id.clone(), repo_type, now_epoch],
                        )
                        .await
                    {
                        eprintln!("db problem: {}", e);
                    }
                }
            }).await;

        transaction.commit().await.unwrap();
        page += 1;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    Ok(())
}

pub async fn codeberg_main_cron(pool: Arc<Connection>) -> Result<(), Box<dyn std::error::Error>> {
    fetch_all_codeberg_repos_cron_updating_part(pool.clone(), "zig-package", true).await?;
    fetch_all_codeberg_repos_cron_updating_part(pool.clone(), "zig", false).await?;
    Ok(())
}

pub async fn run_cron_update_once(pool: Arc<Connection>) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut rows = pool
        .query(
            "SELECT id FROM repo_pipeline_queue WHERE id LIKE 'cb/%' AND status = 'needs_update' ORDER BY queued_at",
            params![],
        )
        .await
        .unwrap();
    let mut repos_that_need_update = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let id: String = row.get(0).unwrap();
        repos_that_need_update.push(id);
    }

    if repos_that_need_update.is_empty() {
        return Ok(());
    }

    for id in repos_that_need_update {
        let parts: Vec<&str> = id.split('/').collect();
        if parts.len() != 3 {
            continue;
        }
        let owner = parts[1];
        let repo = parts[2];

        let url = format!("https://codeberg.org/api/v1/repos/{owner}/{repo}");
        let mut optional_response = None;
        for attempt_count in 0..5 {
            match client
                .get(&url)
                .header("Authorization", &*crate::CODEBERG_KEY)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        optional_response = Some(resp);
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt_count)))
                        .await;
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt_count)))
                        .await;
                }
            }
        }

        if let Some(resp) = optional_response {
            if let Ok(json_body) = resp.text().await {
                if let Ok(daum) = serde_json::from_str::<types::types::Daum>(&json_body) {
                    let repo_data = get_repo_data(daum).await;
                    let transaction = pool.transaction().await.unwrap();
                    send_repo_data_to_database(&transaction, repo_data).await;
                    let now_epoch = chrono::Utc::now().timestamp();
                    transaction
                        .execute(
                            "UPDATE repo_pipeline_queue SET status = 'indexed', processed_at = ? WHERE id = ?",
                            params![now_epoch, id],
                        )
                        .await
                        .unwrap();
                    transaction.commit().await.unwrap();
                } else {
                    eprintln!("failed to parse cb json for {id}");
                }
            }
        }
    }

    Ok(())
}
