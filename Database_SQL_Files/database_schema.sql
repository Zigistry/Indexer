CREATE TABLE users (
  id VARCHAR(45) PRIMARY KEY COLLATE NOCASE,
  avatar_id VARCHAR(65) NOT NULL,
  -- This will now be either gh for GitHub or cb for Codeberg.
  platform VARCHAR(2) NOT NULL COLLATE NOCASE,
  bio VARCHAR(260)
);

CREATE INDEX idx_users_platform ON users (platform);

CREATE TABLE repos (
  -- The username is 40 characters at max, repo name is 100 and the key and slashes is 5
  -- Hence, I can keep this at 150.
  id VARCHAR(150) PRIMARY KEY COLLATE NOCASE,
  -- 40 + the key and the slash i.e 3, so I think best is 45 for htis.
  owner VARCHAR(45) NOT NULL COLLATE NOCASE,
  -- GH CB like, 2 characters
  platform VARCHAR(2) NOT NULL COLLATE NOCASE,
  -- This has afaik, 255, hence, 260
  description VARCHAR(260),
  issues_count INTEGER NOT NULL DEFAULT 0,
  -- This should also be 255, hence, 260
  default_branch_name VARCHAR(260) NOT NULL DEFAULT 'main',
  fork_count INTEGER NOT NULL DEFAULT 0,
  stargazer_count INTEGER NOT NULL DEFAULT 0,
  watchers_count INTEGER NOT NULL DEFAULT 0,
  -- From now, its Epoch
  pushed_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  -- I just realised that the boolean type is like an integer only.
  -- I added check that it should only be either true or false. i.e 0, 1
  is_archived BOOLEAN NOT NULL CHECK (is_archived IN (0, 1)),
  is_disabled BOOLEAN NOT NULL CHECK (is_disabled IN (0, 1)),
  is_fork BOOLEAN NOT NULL CHECK (is_fork IN (0, 1)),
  -- Making this only 30 characters limit, it shouldn't be more than that.
  -- Simple google tells, its just 36 characters, so, ok?
  license VARCHAR(40) COLLATE NOCASE DEFAULT 'NOASSERTION',
  -- Maybe 50 is perfect here
  primary_language VARCHAR(50) COLLATE NOCASE DEFAULT 'Zig',
  latest_commit_hash VARCHAR(50) NOT NULL,
  last_updated_in_this_database INTEGER NOT NULL,
  -- Because the search results are getting too much overload, 
  -- I am trying to make this extremley read intensive.
  latest_release_version VARCHAR(255),
  FOREIGN KEY (owner) REFERENCES users (id) ON DELETE CASCADE
);

CREATE VIRTUAL TABLE repo_search USING fts5 (
    repo_id UNINDEXED,                           -- e.g. "gh/zigzap/zap"
    keywords,
    prefix = '2 3 4',
    tokenize = 'porter unicode61'
);

-- Doing this to make sure, no duplicate repo_id is added.
-- I will be using this query:
-- INSERT OR REPLACE INTO repo_search (repo_id, keywords) VALUES (?, ?)
CREATE TABLE repo_topics (
  repo_id VARCHAR(150) NOT NULL COLLATE NOCASE,
  -- Limit is 50, hence
  topic VARCHAR(60) NOT NULL COLLATE NOCASE,
  PRIMARY KEY (repo_id, topic),
  FOREIGN KEY (repo_id) REFERENCES repos (id) ON DELETE CASCADE,
) WITHOUT ROWID;

CREATE INDEX idx_topics_topic ON repo_topics (topic, repo_id);

CREATE TABLE repo_dependents (
    repo_id VARCHAR(150) NOT NULL COLLATE NOCASE,
    dependent_repo_id VARCHAR(150) NOT NULL COLLATE NOCASE,
    PRIMARY KEY (repo_id, dependent_repo_id),
    FOREIGN KEY (repo_id) REFERENCES repos (id) ON DELETE CASCADE,
    FOREIGN KEY (dependent_repo_id) REFERENCES repos (id) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE releases (
    repo_id VARCHAR(150) NOT NULL COLLATE NOCASE,
    version VARCHAR(255) NOT NULL COLLATE NOCASE,
    is_prerelease BOOLEAN NOT NULL DEFAULT 0,
    published_at INTEGER NOT NULL, -- Epoch
    minimum_zig_version VARCHAR(30),
    readme_url TEXT,
    PRIMARY KEY (repo_id, version),
    FOREIGN KEY (repo_id) REFERENCES repos (id) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE INDEX idx_releases_repo_publish ON releases (repo_id, published_at DESC);

CREATE TABLE release_dependencies (
    repo_id VARCHAR(150) NOT NULL COLLATE NOCASE,
    version VARCHAR(255) NOT NULL COLLATE NOCASE,
    name VARCHAR(260) NOT NULL COLLATE NOCASE,
    hash VARCHAR(260) NOT NULL,
    is_lazy BOOLEAN NOT NULL DEFAULT 0,
    url VARCHAR(260) NOT NULL,
    path VARCHAR(260),
    PRIMARY KEY (repo_id, version, name),
    FOREIGN KEY (repo_id, version) REFERENCES releases (repo_id, version) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE index_sections (
    section_name VARCHAR(20) NOT NULL COLLATE NOCASE,
    repo_id VARCHAR(150) NOT NULL COLLATE NOCASE,
    PRIMARY KEY (section_name, repo_id),
    FOREIGN KEY (repo_id) REFERENCES repos (id) ON DELETE CASCADE
) WITHOUT ROWID;

-- For this specifically, I have created a fasttext validation, I will train it to detect
-- all the scam repos, if my AI algorithm flags a repo
-- to review it will be added to this table as quarantined, if no problem, it will continue.
CREATE TABLE repo_pipeline_queue (
    id VARCHAR(150) PRIMARY KEY COLLATE NOCASE,
    type_of_repo VARCHAR(10) NOT NULL,           -- 'package' or 'program'
    status VARCHAR(20) NOT NULL DEFAULT 'pending_check', -- 'pending_check', 'safe', 'quarantined', 'needs_update', 'indexed'
    reason TEXT,
    queued_at INTEGER NOT NULL,
    processed_at INTEGER
);




CREATE INDEX idx_pipeline_status ON repo_pipeline_queue (status, queued_at);

-- These are only the users who are either spamming or
-- shipping malware.
CREATE TABLE banned_users (
    id VARCHAR(45) PRIMARY KEY COLLATE NOCASE,
    banned_at INTEGER NOT NULL,
    reason TEXT
);

-- This is for making it easier to fetch
-- details of these repositories
-- to improve the accuracy of the algorithm
-- which detects such repos.
CREATE TABLE banned_repos (
    id VARCHAR(150) PRIMARY KEY COLLATE NOCASE,
    banned_at INTEGER NOT NULL,
    reason TEXT
);

