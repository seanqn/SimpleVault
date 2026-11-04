-- Schema from legacy/data/repositories/db_local.cpp; Qt left foreign_keys disabled.
CREATE TABLE groups (id BLOB PRIMARY KEY, name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE vault_content (group_id BLOB, content_id BLOB PRIMARY KEY,
    org_name TEXT, username TEXT, password TEXT, email TEXT, note TEXT,
    FOREIGN KEY(group_id) REFERENCES groups(id) ON DELETE CASCADE) WITHOUT ROWID;
