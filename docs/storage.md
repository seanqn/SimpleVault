# Rust SQLite repository

`vault-core::repository::VaultRepository` is the application-facing interface.
`vault-storage::SqliteRepository` implements it with rusqlite and bundled SQLite.
The adapter does not import or build anything in `legacy/`. Bundled SQLite is C
and needs a C compiler during the first build; the old C++/Qt application is not a
build or runtime dependency. Rust 1.85 is the declared minimum; this phase was
validated with Rust 1.99.1 on Linux.

## Opening a vault

The controller obtains an owned `SecretKey` from its `KeyProvider` and uses
`SqliteRepository::open_existing(path, key)` through the `SqliteVault` factory.
That path refuses missing or uninitialized vaults. Explicit provisioning uses
`SqliteRepository::open(path, key)`. The caller selects a private
application-data directory; the adapter does not locate or open a user's real
vault automatically. Parent directories must already exist. A new empty database
is initialized at `PRAGMA user_version = 1` within a transaction. An existing v1
database must pass schema, integrity, foreign-key, and encrypted key-check tests.
Wrong keys are rejected before returning a repository that can write data.

Keep the repository alive only while the vault is unlocked. Dropping it closes
the SQLite connection and drops its zeroizing key. There is no connection pool,
UI, background process, or OS keychain adapter in this phase.

## Operations and invariants

- Group and credential IDs are 16-byte RFC 4122 UUID BLOBs, matching Qt's
  `QUuid::toRfc4122`. The application supplies IDs; storage does not generate them.
- Group create/list/rename/delete and credential list/upsert/delete use bound
  parameters. Missing targets return `NotFound`, including missing parent groups.
- A credential belongs to one group. Saving its ID under another group returns
  `GroupConflict`; deleting under the wrong group cannot remove it.
- `save_credentials` upserts an entire batch in an immediate transaction. Any
  failure rolls back previous updates and inserts in the batch. The application
  must retain its edit cache until this method succeeds.
- Every connection enables foreign keys. Deleting a group cascades to its rows.
- Queries use deterministic ordering: groups by creation timestamp and ID,
  credentials by ID. SQL failures remain errors rather than empty query results.

## Encryption boundary

Only passwords are encrypted. Each save uses a fresh 24-byte random nonce and
XChaCha20-Poly1305 ciphertext including the 16-byte authentication tag, stored as
separate BLOB columns. The phase 2 no-associated-data format is retained; this
format does not cryptographically bind ciphertext to a particular row ID.
Group names, organization names, usernames, email addresses, notes, and timestamps
are plaintext. This is not whole-database encryption.

`encrypt_stored_password` accepts all existing values, including empty and short
passwords. The original `encrypt` entry point still enforces its legacy six-UTF-16-
unit rule. Password-strength policy is a controller concern, not a storage rule.
Service-owned temporary plaintext is zeroized. Returned `Credential` values contain
ordinary strings; the controller now wraps temporary rows in `Zeroizing`, and the
edit cache clears its owned fields on replacement/reset/drop. SQLite and callers
may retain other copies outside this erasure guarantee.

## Legacy migration and recovery

Inspection of `legacy/data/repositories/db_local.cpp` confirms that the old
`password TEXT` column holds plaintext: its repository did not call CryptoServicer.
An ordinary `open` returns `MigrationRequired` for nonempty version-zero databases.
Migration must be explicit:

```text
SqliteRepository::migrate_legacy(source_path, unused_backup_path, owned_key)
```

Close the old application before migrating and stop using it against the migrated
database. It cannot read the new schema. Only the recognized version-zero Qt
schema is accepted; unknown versions/layouts, triggers, and views are rejected.

Migration reserves the writer, then uses SQLite's online backup API through a
separate read connection. This includes committed WAL contents, unlike copying
only the `.sqlite` file. The backup is flushed to disk and published without
overwriting an existing destination. On Unix it is created with mode 0600 and
the parent directory is synced before modifying the source schema.

A single transaction rebuilds tables, preserves UUID bytes and creation times,
encrypts passwords, adds a key check, and advances `user_version` to 1. Null text
fields become empty strings (matching Qt reads); null creation times receive the
migration timestamp. Orphan rows and malformed IDs cause rollback, not deletion.
A failure after backup creation retains that backup and restores the original
schema/data transactionally. A second migration of v1 is rejected; ordinary
reopening is repeatable and does not migrate or generate another backup.

The backup still contains plaintext passwords. Retain it securely until migration
has been reviewed. For recovery, close all connections and restore the backup to
a separate path before opening or retrying migration. Do not overwrite an active
database or mix it with leftover WAL/SHM files. The migration enables SQLite
`secure_delete`, but does not promise removal of historical plaintext from old
WAL files, backups, filesystem snapshots, or storage media.

No real user database was migrated during implementation. Tests use temporary
files containing a fixture of the original Qt schema.

## Validation

From the repository root:

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Storage tests exercise persistence after reopen, raw UUID BLOBs, encrypted password
storage, CRUD, Unicode, parameter binding, group isolation, batch rollback,
cascades, wrong keys, corrupted ciphertext, WAL-aware backups, nullable/short legacy
values, failed migrations, malformed IDs, unknown schemas, and backup protection.
