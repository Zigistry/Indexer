-- These are only intended to run for backend API.

-- packages
CREATE INDEX idx_repos_pkg_stars ON repos (
    is_disabled, is_package, stargazer_count DESC, id ASC
);
CREATE INDEX idx_repos_pkg_dependents ON repos (
    is_disabled, is_package, dependents_count DESC, id ASC
);
CREATE INDEX idx_repos_pkg_pushed ON repos (
    is_disabled, is_package, pushed_at DESC, id ASC
);
CREATE INDEX idx_repos_pkg_created ON repos (
    is_disabled, is_package, created_at DESC, id ASC
);
-- programs
CREATE INDEX idx_repos_prog_stars ON repos (
    is_disabled, is_program, stargazer_count DESC, id ASC
);
CREATE INDEX idx_repos_prog_dependents ON repos (
    is_disabled, is_program, dependents_count DESC, id ASC
);
CREATE INDEX idx_repos_prog_pushed ON repos (
    is_disabled, is_program, pushed_at DESC, id ASC
);
CREATE INDEX idx_repos_prog_created ON repos (
    is_disabled, is_program, created_at DESC, id ASC
);
-- users
CREATE INDEX idx_repos_owner_stars ON repos (
    owner, is_disabled, stargazer_count DESC
);

CREATE INDEX idx_releases_repo_publish ON releases (repo_id, published_at DESC);

CREATE INDEX idx_topics_topic ON repo_topics (topic, repo_id);

CREATE INDEX idx_users_platform_id ON users (platform_id);

CREATE INDEX idx_pipeline_status ON repo_pipeline_queue (status, queued_at);

