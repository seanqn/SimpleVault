# Rust controller integration

`vault-app` composes the independent core and SQLite adapter. The code under
`legacy/` is reference material only. The application layer is a synchronous Rust
library ready for the future desktop command host; it does not launch a GUI.

## Responsibilities

- `VaultController<R: VaultRepository>` handles group operations, ID selection,
  provisional credentials, editing, explicit password reveal, and persistence.
- `VaultApplication<F: VaultFactory>` owns the locked/unlocked/closed lifecycle.
- `SqliteVault` is the production factory for an existing SQLite vault. Tests use
  the same adapter with injected failures, not an in-memory replacement for SQL.
- `KeyProvider` supplies an owned key at unlock. The application does not retain
  the provider or another key copy. The opened repository owns the key until lock.

The host should own one application instance and serialize commands through it.
Group and credential operations take stable 16-byte UUID IDs rather than list
indices. New IDs are generated as UUID v7 values. Listing uses live repository
reads; there is no second model cache to reconcile after a committed write.
Startup only validates access and loads no mock data.

## Unlock and provisioning

Construct `VaultApplication::new(SqliteVault::new(path))`; it starts `Locked`.
`unlock(&provider)` obtains a key, opens the existing database, verifies the storage
key check, and tests the initial group read before publishing an unlocked controller.
A failure at any step leaves the application locked and releases intermediate
resources. Unlock while unlocked or closed is rejected before invoking the provider.

Unlock uses `SqliteRepository::open_existing`: neither a missing file nor an empty
uninitialized file becomes a new vault. Provisioning with a securely managed key
through `SqliteRepository::open`, and backed-up legacy migration through
`migrate_legacy`, remain explicit storage operations. See [storage.md](storage.md).

This is key-based session orchestration. It does not implement master-password
verification/derivation, account registration/deletion, key generation/storage, or
macOS keychain prompts. The host/key-provider implementation must authenticate the
user before releasing a key; the native adapter is phase 6. No plaintext-password
comparison from the legacy authentication prototype was carried forward.

## Editing and selection rules

Use `controller()` or `controller_mut()` only while unlocked. Locked/closed access
returns a typed error, with no controller or repository exposed.

| Command | Result |
| --- | --- |
| `create_group(name)` / `rename_group(id, name)` | Trim names, reject blank names, propagate repository errors. |
| `select_group(Some(id))` / `select_credential(Some(id))` | Validate target first, finish the current edit, then change selection. |
| `select_group(None)` / `select_credential(None)` | Finish the edit before clearing selection. |
| `begin_edit(id)` | Validate row membership, finish a previous edit, then own an original/draft pair. |
| `add_credential()` | Start one provisional row in the selected group; reuse an empty provisional row. Save a filled row before creating another. |
| `update_edit(field, value)` | Change a typed field; `update_edit_role` accepts the established field names. |
| `submit_edit()` | Return a typed outcome; clear the edit only after the required write succeeds. |
| `cancel_edit()` | Discard the draft. Remove a provisional row's selection; preserve an existing row's selection. |

Selecting the same group/credential or beginning the same active edit is a no-op
and does not replace a dirty draft. Invalid targets and failed target reads do not
save the current edit. If saving fails, selection and the full draft remain intact
for retry; navigation, adding another row, and save-on-lock/quit stop at that error.

Submission outcomes:

- `NoEdit`: no session was active.
- `Unchanged`: an existing row's fields match its original; finish without writing.
- `DiscardedEmptyNewRow`: remove an empty provisional row without a database write.
- `Saved`: the repository accepted the draft; clear the cache and provisional flag.

An existing row edited to all-empty fields is **saved**, not implicitly deleted.
Deletion requires an explicit command. This prevents accidental data deletion and
makes the behavior consistent with phase 3's support for empty stored passwords.

## Deletion

`remove_group(id)` and `remove_credential(id)` change local selection/edit state
only after successful persistent deletion. A failed deletion retains the draft.
Deleting another group or row does not disturb the active edit. Successful deletion
of the edited row or its group explicitly discards that draft; it does not first
save data that is being deleted. Group deletion uses the repository's cascade.
Deleting a provisional row only cancels that local row, without touching SQLite.

## Frontend boundary and secret ownership

`credentials()` returns `CredentialSummary` values with IDs and display metadata,
never a password field. Summaries overlay the current draft and include a provisional
row so the future frontend can render pending work. `reveal_password(id)` explicitly
returns an owned zeroizing string, scoped to the selected group; it reflects an
active draft when applicable. The frontend adapter must clear any copies it creates.

`draft()` is backend access for editing, not a response to serialize wholesale.
No keys or repositories are returned by controller commands. Group names, usernames,
email addresses, and notes remain sensitive display data and should not be logged.
Error display strings are generic; typed inner errors remain available for host
mapping, and should not be dumped into frontend responses or logs blindly.

The edit cache now zeroizes its owned original/draft fields when replaced, reset,
or dropped. Replaced and rejected input values are cleared, and fetched temporary
credential collections in the controller use `Zeroizing`. The repository's key
is released when the session closes. This does not erase arbitrary caller copies,
SQLite internal allocations, frontend strings, or storage media.

## Lock and shutdown

Both `lock(resolution)` and `shutdown(resolution)` require an explicit choice:

| Resolution | Behavior |
| --- | --- |
| `Save` | Submit the active edit; a failure keeps the unlocked session and draft for retry. |
| `Discard` | Cancel the draft and immediately release the controller/repository/key. |
| `Cancel` | Return `Cancelled` without changing the session. |

A completed lock returns to `Locked`, allowing a new key-provider unlock with clean
selection state. A completed shutdown becomes `Closed`; further unlocks are rejected.
Repeated shutdown is harmless. For an immediate security lock (for example, a future
system-lock event), choose `Discard` explicitly rather than leaving a failed-save
session unlocked. User-initiated save/quit can display the error and let the user retry.

Dropping the application performs no persistence. The future host must call shutdown
and honor its outcome before exiting; there is no fallible save hidden in a destructor.

## Validation

Run the workspace's locked tests, formatting check, and Clippy from the repository
root. Controller tests cover actual SQLite CRUD, ID selection, edit outcomes,
provisional rows, failed saves/reads/deletes, retries, group isolation, password
reveal, key failures, repository release, and save/discard/cancel lock/quit flows.
The Rust library has been validated on Linux. UI behavior and native macOS events
are not exercised until their later phases.
