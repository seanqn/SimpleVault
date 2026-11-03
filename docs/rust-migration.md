# Incremental Rust migration

Source baseline: `feature/macos-agent`, commit
`5df2f9262de6dac23d9b64ff0267b34f83d32317`.

## Target architecture

Use a platform-independent Rust domain/service library, SQLite repository adapters,
a small Tauri desktop host, and a TypeScript web frontend. Tauri is the proposed
host for later phases; it is not installed or integrated in this first slice.
Keep crypto, database access, keys, and commit decisions in Rust. Expose explicit
commands and safe response types to the frontend, never raw keys or repository handles.
Keep platform lifecycle and tray positioning out of the domain library.

The existing C++/QML app remains the reference until replacement parity is verified.
The Rust library currently runs only in tests; it does not replace the running Qt app.

## Small reviewable phases

1. **Domain and edit cache (this change).** Preserve 16-byte credential IDs and
   the five editable fields. Replace string dispatch internally with typed fields.
   Keep the existing role names available through checked conversion. Test inactive,
   clean, dirty, empty, revert, Unicode, identity, and reset behavior. Reset also
   clears provisional-row state. No encryption or persistence is claimed here;
   ordinary Rust strings do not provide guaranteed memory zeroization.
2. **Crypto service.** Preserve XChaCha20-Poly1305's 32-byte key, 24-byte nonce,
   16-byte tag and existing no-associated-data format. Add cross-language fixtures,
   wrong-key and tampering tests before replacing libsodium calls. Define secret
   ownership/zeroization and key-provider interfaces before connecting persistence.
3. **SQLite and repository.** Use temporary databases to verify group/credential
   CRUD, UUID BLOB compatibility, transactions, and foreign-key cascades. Back up
   existing databases before versioned migrations. Investigate plaintext password
   storage in the current schema rather than assuming crypto is already wired in.
4. **Application/controller.** Split VaultManager into group operations, edit
   sessions, selection state, and authentication orchestration. Use stable IDs at
   the command boundary rather than UI row indices. A failed save must retain the
   draft and block navigation; clean/empty/save paths must explicitly finish the
   session. Test transitions, rollback, deletion of selected groups, lock, and quit.
   Do not seed mock groups in production startup.
5. **Frontend and desktop host.** Port QML flows to TypeScript components backed
   by typed commands. Preserve the 700×600 white authentication window, centered
   250×300 modal, and UID footer. Preserve the 320×400 tray popup, #2B2B2B surface,
   #3D3D3D 1px border, 12px corners, 4px anchor offset and dismiss-on-blur behavior.
   `Home.qml` currently contains an empty tray surface; `HomePane.qml` contains a
   prototype group list, not a complete populated vault. Match implemented styling
   and distinguish new UI from parity work. Capture macOS reference screenshots
   before replacing platform-native Qt controls whose appearance is implicit.
6. **macOS integration and cutover.** Implement keychain, tray lifecycle, activation,
   display scaling/multiple screens, and packaging in platform adapters. Validate
   on macOS with the real keychain and signed bundle; Linux unit tests cannot
   establish this behavior. Remove Qt only after end-to-end parity and data migration
   checks pass.

## Known legacy issues to address during the relevant phase

- `renameGroup` prepares `:groupID` without binding it.
- Foreign-key enforcement is commented out in SQLite initialization.
- Controller submit returns early for empty/clean edits without finishing the edit;
  persistence failure is not checked before resetting the draft.
- The CMake configuration has developer-specific Qt paths and a missing QML test
  input, and links macOS frameworks unconditionally.

These findings are not patched by the first service slice.

## Validation

From the repository root, with a Rust toolchain on PATH:

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No GUI, production data, credentials, Qt installation, or network access is required
for the current crate. Each later phase should add its own behavioral checks while
keeping these core tests fast. Build desktop bundles only when a phase needs them.
