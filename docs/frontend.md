# Frontend and desktop development

The React/TypeScript frontend calls `vault-bridge` through one Tauri command,
`vault_command`. The bridge owns `VaultApplication<SqliteVault>` and a `KeyProvider`;
crypto, SQLite access, session transitions, and commit decisions stay in Rust.
Neither the frontend nor desktop host builds or links anything under `legacy/`.

## Run the desktop development vault

Install Node.js 20.19+ (or a current LTS), a C compiler, and Rust 1.90+ for the
locked desktop dependency set. The core workspace still declares Rust 1.85+.
On macOS, install Xcode Command Line Tools (`xcode-select --install`). On
Debian/Ubuntu, install the Tauri native prerequisites:

```sh
sudo apt-get install build-essential pkg-config libgtk-3-dev \
  libwebkit2gtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev
```

From the repository root:

```sh
npm --prefix frontend ci
npm --prefix frontend run desktop:dev
```

This command explicitly enables `dev-vault`. Unlock opens an initially empty
temporary SQLite vault. Create a group, select it, then add credentials. Its
random encryption key stays in Rust; lock/unlock preserves the temporary vault
within the running process. Successful quit releases the connection/key and
removes the temporary directory. Use test data only: restarting creates a new
vault, and abrupt process termination may leave an unusable temporary database.

The default host has no native key provider yet. A normal build opens the locked
screen and reports unavailable device authentication on unlock. It does not
initialize or migrate a user's database. Phase 6 must implement key provisioning,
authentication, and a reviewed migration entry point before production use.

```sh
npm --prefix frontend run desktop:build
```

This builds the default host with embedded frontend assets. Distribution bundles
are disabled in `desktop/tauri.conf.json` until platform packaging is implemented.
For a build with the disposable vault, use:

```sh
npm --prefix frontend run desktop -- build --features dev-vault
```

`npm --prefix frontend run dev` serves the frontend alone on loopback port 1420;
a normal browser displays the desktop-app message because it has no vault IPC.
There is no browser-accessible vault server or fake populated production startup.

## Views and command behavior

- Authentication: white 700×600 window, centered 250×300 card, UID footer.
- Vault: 320×400 dark panel, #2B2B2B surface, 1px #3D3D3D border, 12px CSS corners.
- Groups: create, select, rename, and confirmed deletion with cascade behavior.
- Credentials: searchable list, organization/username/email/password/notes editor,
  explicit deletion, and empty states. These populated views extend the unfinished
  legacy tray UI rather than claiming pixel parity with nonexistent screens.
- Lock and quit: save/discard/cancel when editing. Native close requests also
  reach the confirmation dialog. Failed writes preserve the editor for retry.

`frontend/src/protocol.ts` mirrors the Rust request/response schema. Requests
use stable UUIDs; malformed IDs and unknown fields are rejected by Rust. Regular
snapshots omit passwords. An omitted password in `update_draft` preserves its
current value; an explicit empty string clears it. Revealing requires its own
command and hides on blur, visibility changes, other actions, or after 15 seconds.
The frontend clears secret references on session/edit transitions and stores
nothing in browser storage. JavaScript and IPC copies cannot promise memory
zeroization; see the core/storage documentation for Rust-side guarantees.

Buttons are disabled while a command is pending to prevent duplicate submits.
If a mutation succeeds but reading the updated view fails, `refresh_error`
requests a snapshot refresh rather than retrying the mutation. Errors contain
sanitized client messages, not raw database failures. The desktop CSP permits
local assets and IPC; capabilities allow event subscription and window dragging.
There are no shell, filesystem, or external-network frontend capabilities.

## Validation

From the repository root:

```sh
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build -p vault-bridge --bin browser-harness --features dev-vault --locked
npm --prefix frontend run format:check
npm --prefix frontend run build
npm --prefix frontend test
```

Browser tests use `/usr/bin/chromium` by default. Set `CHROMIUM_PATH` to your
Chromium/Chrome executable on another system. `VAULT_HARNESS` can override the
compiled harness path (for example, a Windows `.exe`). Tests start Vite, attach a
test IPC transport to each page, and launch a fresh Rust process with real SQLite
storage per test. Only transport failures/delays are injected; CRUD and crypto
run through the production controller/storage code. Browser tests need permission
to bind a loopback port and launch Chromium. Container tests disable Chromium's
sandbox; desktop runtime configuration is independent of this test setting.

With native libraries installed, also run:

```sh
cargo fmt --manifest-path desktop/Cargo.toml -- --check
cargo check --manifest-path desktop/Cargo.toml --locked
cargo clippy --manifest-path desktop/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo build --manifest-path desktop/Cargo.toml --features dev-vault,custom-protocol --locked
```

The final build embeds assets from the preceding frontend build. The separate
desktop Cargo project keeps GTK/WebKit dependencies out of ordinary core tests.

Cloud validation passed 34 Rust tests, five browser integration tests, frontend build
and formatting, and Linux desktop compilation/linting. Browser coverage includes
styles, CRUD, search, password preservation/reveal, lock/unlock, failed-save retry,
deletion confirmations, duplicate-submit prevention, and keyboard/close events.
Screenshots are generated under `frontend/test-results/` and ignored by Git.

Native startup was attempted with verified native packages extracted under a
workspace-local prefix. WebKit required system-path helpers; the attempted path
mapping then terminated during startup. Consequently native IPC, window resizing,
and WebKit rendering have not been verified by this cloud run. Use the standard
platform installation above for a native smoke test.

## Remaining macOS integration

Phase 6 still needs the keychain provider and real UID, tray activation and the
4px anchor offset, dismissal on blur, native transparent corners, multiple
displays/scaling, platform screenshots, and signing/packaging. CSS dimensions and
browser close-event tests do not verify those native behaviors. Keep `legacy/`
until the real macOS end-to-end and data migration checks pass.
