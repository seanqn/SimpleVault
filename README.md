# SimpleVault

Incremental migration of the local credential vault to Rust and a React/TypeScript frontend.
The previous Qt/C++ application is retained under `legacy/` for reference.

- `rust/vault-core`: credential domain, edit cache, crypto, and repository interface.
- `rust/vault-storage`: SQLite adapter with encrypted passwords and backed-up legacy migration.
- `rust/vault-app`: controller commands, editing, and key-based session lifecycle.
- `rust/vault-bridge`: typed commands and password-free view models for the frontend.
- `frontend`: React views, styling, and browser integration tests against Rust/SQLite.
- `desktop`: Tauri host (separate Cargo project; requires Rust 1.90+ and native GUI libraries).
- [Frontend setup, development vault, and validation](docs/frontend.md)
- [Controller API and lifecycle](docs/controller.md)
- [Migration plan](docs/rust-migration.md)
- [Storage API, migration, and recovery](docs/storage.md)

From this directory, with Rust and a C compiler installed:

```sh
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

The Rust workspace and desktop frontend do not depend on the legacy C++ application.
The frontend can be exercised with an explicitly enabled disposable development vault:

```sh
npm --prefix frontend ci
npm --prefix frontend run desktop:dev
```

See the frontend guide for platform prerequisites. The development vault is cleared
on quit and must only contain test data. Normal builds remain locked until phase 6
adds native key provisioning/authentication. Menu-bar positioning, native macOS
validation, and signed packaging also remain for phase 6.

## Historical project notes


Secure local storage and encryption for credentials with a
simplified, user-friendly interface.
####

### Debug - App is a work in progress

### Authentication Prototype
Uses macOS keychain via qtkeychain. A single master password is input on first build that future sign-ins will use. UID is tied
to the machine when the app is first built; changing master password is not supported at the moment. Future versions will likely aim
for a auth flow that is purely attached to the interface instead of depending on keychain. 

qtkeychain handles bridging between keychain and app. Backend auth logic which implements qtkeychain can be found in /services.

### Vault/Home/Groups Prototype
The home view serves as the anchor for rest of the UI after successful authentication. A Groups sidebar allows custom groups or
"vaults" to be added or removed.

### Future Additions
New features to be added will be built on the previously listed core features. Vault content will display clean and organized
lines or boxes of text that make it easy to identify usernames, passwords, organization names, and other credentials. An account
page is a necessary inclusion that will reflect the user's information and preferences.
####
####
### Archive
Custom cross-language bridges were made in earlier builds which intended to resolve application input to macOS keychain storage
and retrieval. These bridges were near fully functional but a working implementation required subscription to Apple SDK. Current
builds depend on qtkeychain to work around this.

### Architecture Changes
Refactoring towards an improved MVVM structure with a more focused
controller (VaultManager) and a repository for dedicated database
communication.

Turned the database side read operations into a generic templated function

Added repository side lambda-based methods to read from database tables by currying

Updated controller logic
