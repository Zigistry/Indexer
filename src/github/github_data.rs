use crate::custom_types::Dependency;
use crate::github::types::Node;
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseData {
    pub tag_name: String,
    pub is_prerelease: bool,
    pub published_at: String,
    pub minimum_zig_version: String,
    pub readme_url: String,
    pub dependencies: Vec<Dependency>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoData {
    pub repository: Node,
    pub is_package: bool,
    pub user_id: String,
    pub repo_id: String,
    pub readme_url: String,
    pub readme_content: String,
    pub build_zig_zon_version: String,
    pub build_zig_zon_dependencies: Vec<Dependency>,
    pub releases: Vec<ReleaseData>,
}
