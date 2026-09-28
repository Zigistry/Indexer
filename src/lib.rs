/*!
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-Zigistry-Database-Permission
 *
 * Copyright (c) 2025 Rohan Vashisht
 *
 * This software is licensed under the GNU Affero General Public License v3.0.
 * The database content under `zigistry/indexer` is subject to additional terms.
 *
 *      ______       _     _
 *     |__  (_) __ _(_)___| |_ _ __ _   _
 *       / /| |/ _` | / __| __| '__| | | |
 *      / /_| | (_| | \__ \ |_| |  | |_| |
 *     /____|_|\__, |_|___/\__|_|   \__, |
 *             |___/                |___/
 *
 *
 * See LICENSE and LICENSE-ADDITIONAL in the project root directory for full details.
 */

pub mod bzz_stuff;
pub mod codeberg;
pub mod constants;
pub mod custom_types;

pub mod database;
pub mod github;

use serde::Deserialize;
use serde_json::json;
use std::env;
use std::sync::LazyLock;

pub static GITHUB_KEY: LazyLock<String> = LazyLock::new(|| {
    "Bearer ".to_string() + &env::var("GH_API_KEY").expect("GH_API_KEY not set")
});
pub static CODEBERG_KEY: LazyLock<String> = LazyLock::new(|| {
    "token ".to_string() + &env::var("CB_API_KEY").expect("CB_API_KEY not set")
});

#[derive(Deserialize)]
struct ReadmeKeywords {
    keywords: String,
}

pub async fn keyword_extraction(
    readme_content: &str,
    description: &str,
    repo_name: &str,
    owner_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://127.0.0.1:8000/extract_keywords")
        .json(&json!({
            "text": format!("{readme_content} {description}")
        }))
        .send()
        .await?;
    let keywords = res.json::<ReadmeKeywords>().await?;
    Ok(format!("{repo_name} {owner_name} {}", keywords.keywords))
}
