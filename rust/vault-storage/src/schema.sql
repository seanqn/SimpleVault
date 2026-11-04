CREATE TABLE groups (
    id BLOB PRIMARY KEY NOT NULL CHECK(typeof(id) = 'blob' AND length(id) = 16),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE vault_content (
    content_id BLOB PRIMARY KEY NOT NULL CHECK(typeof(content_id) = 'blob' AND length(content_id) = 16),
    group_id BLOB NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    org_name TEXT NOT NULL,
    username TEXT NOT NULL,
    password_nonce BLOB NOT NULL CHECK(typeof(password_nonce) = 'blob' AND length(password_nonce) = 24),
    password_ciphertext BLOB NOT NULL CHECK(typeof(password_ciphertext) = 'blob' AND length(password_ciphertext) >= 16),
    email TEXT NOT NULL,
    note TEXT NOT NULL
) WITHOUT ROWID;
CREATE INDEX vault_content_group ON vault_content(group_id);
CREATE TABLE vault_metadata (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    nonce BLOB NOT NULL CHECK(length(nonce) = 24),
    ciphertext BLOB NOT NULL CHECK(length(ciphertext) >= 16)
);
PRAGMA user_version = 1;
