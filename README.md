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
