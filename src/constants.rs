pub const ASYNC_LIMIT: usize = 500;
pub const GH_GRAPH_QL_QUERY: &str =
    include_str!("../GitHub_GQL_API_Files/fetch-search-query-everything.gql");
pub const GH_GRAPH_QL_PARTIAL_QUERY: &str =
    include_str!("../GitHub_GQL_API_Files/fetch-search-query-partial.gql");
pub const GH_GRAPH_QL_100_REPOS_FRAGMENT: &str =
    include_str!("../GitHub_GQL_API_Files/fetch-100-repos.gql");
pub const NEEDS_UPDATE_CHUNK_SIZE: usize = 50;
// These are the amount of characters allowed
// for every feild in the database, I am adding
// this to make sure that I don't exceed the length
// and cause potential problems:

pub mod limits {
    pub const USER_ID_MAX_LEN: usize = 45;
    pub const USER_AVATAR_ID_MAX_LEN: usize = 65;
    pub const PLATFORM_MAX_LEN: usize = 2;
    pub const USER_BIO_MAX_LEN: usize = 260;
    pub const REPO_ID_MAX_LEN: usize = 150;
    pub const REPO_OWNER_MAX_LEN: usize = 45;
    pub const REPO_DESCRIPTION_MAX_LEN: usize = 260;
    pub const REPO_DEFAULT_BRANCH_MAX_LEN: usize = 260;
    pub const REPO_LICENSE_MAX_LEN: usize = 40;
    pub const REPO_PRIMARY_LANGUAGE_MAX_LEN: usize = 50;
    pub const REPO_COMMIT_HASH_MAX_LEN: usize = 40;
    pub const TOPIC_MAX_LEN: usize = 60;
    pub const DEPENDENT_MAX_LEN: usize = 150;
    pub const RELEASE_VERSION_MAX_LEN: usize = 255;
    pub const RELEASE_MIN_ZIG_VERSION_MAX_LEN: usize = 30;
    pub const RELEASE_DEPENDENCY_FIELD_MAX_LEN: usize = 260;
    pub const INDEX_SECTION_NAME_MAX_LEN: usize = 20;
    pub const RELEASE_DIRECTORY_FILES_MAX_LEN: usize = 5000;
}
