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

1. **Domain and edit cache (completed).** Preserve 16-byte credential IDs and
   the five editable fields. Replace string dispatch internally with typed fields.
   Keep the existing role names available through checked conversion. Test inactive,
   clean, dirty, empty, revert, Unicode, identity, and reset behavior. Reset also
   clears provisional-row state. No encryption or persistence is claimed here;
   ordinary Rust strings do not provide guaranteed memory zeroization. Phase 4
   adds explicit clearing for the cache and controller-owned temporary values.
2. **Crypto service (completed).** Preserve XChaCha20-Poly1305's 32-byte key, 24-byte nonce,
   16-byte tag and existing no-associated-data format. Add cross-language fixtures,
   wrong-key and tampering tests before replacing libsodium calls. Define secret
   ownership/zeroization and key-provider interfaces before connecting persistence.
3. **SQLite and repository (completed).** Use temporary databases to verify group/credential
   CRUD, UUID BLOB compatibility, transactions, and foreign-key cascades. Back up
   existing databases before versioned migrations. Investigate plaintext password
   storage in the current schema rather than assuming crypto is already wired in.
4. **Application/controller (completed).** Split VaultManager into group operations, edit
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

The Rust storage adapter fixes the rename binding and foreign-key enforcement;
the legacy source remains a reference. The Rust controller now finishes all edit
paths explicitly and retains drafts on failed saves.

## Validation

From the repository root, with a Rust toolchain on PATH:

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No GUI, production data, credentials, or Qt installation is required for the tests.
The storage adapter builds bundled SQLite with a C compiler. Initial dependency
installation requires registry access; cached test runs work offline. Each later phase should add its own behavioral checks while
keeping these core tests fast. Build desktop bundles only when a phase needs them.

## Phase 2 implementation status

Phase 1 was checked against its requirements and all three edit-cache tests passed.
The Rust crypto module now implements the legacy XChaCha20-Poly1305 format with
random OS-generated nonces and empty associated data. It preserves the six UTF-16
code-unit encryption minimum used by QString. Malformed decrypted UTF-8 is rejected
rather than replaced with replacement characters. Nonce length is enforced by type.

Keys and owned plaintext buffers use zeroizing storage; neither SecretKey nor the
edit cache implements Debug. Caller copies and existing Credential strings remain
outside the crypto service's erasure guarantee. KeyProvider defines the adapter
boundary; real keychain access and persistence wiring remain later phases.

The public fixture uses key bytes 7, nonce bytes 9, and plaintext `secret🔐`.
Regenerate it with `python scripts/crypto_fixture.py` on a machine with libsodium.
Rust encryption matches the C output byte for byte, and Rust decrypts that output.
Tests also cover wrong keys, modifications to every ciphertext/tag byte, nonce
modification, short ciphertext, invalid key size, password length and Unicode.
Six tests pass in total. Rust 1.85 or newer is required by the dependency set;
validation used the installed Rust 1.99.1 toolchain. Initial dependency installation
requires registry access; fixture regeneration requires libsodium, normal tests do not.

## Phase 3 implementation status

The reference application now lives in `legacy/`. The independent `vault-storage`
crate implements the core `VaultRepository` interface with encrypted-password
SQLite persistence. Version-one schema initialization, explicit backed-up legacy
migration, CRUD, atomic batch saves, and cascades are implemented. Legacy passwords
were confirmed to be plaintext; migration encrypts them with a supplied key and
preserves even empty/short values. A key check prevents opening an encrypted vault
for writes under the wrong key.

See [storage contracts and migration procedure](storage.md) for API behavior,
backup/recovery details, and encryption limits. The phase 4 application layer now wires
this adapter to key acquisition and the edit-cache lifecycle. No production database
was touched.

## Phase 4 implementation status

The independent `vault-app` crate integrates `VaultController`, session lifecycle,
the existing `KeyProvider` interface, and the production SQLite factory. Group and
credential commands use stable UUID IDs. Selection changes preflight the target and
finish editing before changing state; save failures retain the draft and block the
transition. Blank new rows are discarded, unchanged edits finish without writes,
and clearing an existing row saves its values rather than implicitly deleting it.

Unlock validates an existing database and its key before exposing the controller.
Lock/quit support explicit save, discard, or cancel, and no persistence runs during
Drop. Successful lock releases the repository/key and clears cache-owned secrets.
Startup seeds no mock data. Controller integration tests use temporary SQLite files
and injected failures to verify the state transitions and retry behavior.

See [controller commands and lifecycle](controller.md). Frontend rendering and
command serialization are phase 5; master-password/key provisioning and native
keychain authentication are not implemented by this phase's key-based orchestration.
