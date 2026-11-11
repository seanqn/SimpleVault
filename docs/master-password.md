# Master-password vaults

The production desktop host creates one persistent vault at its application-data `vault.sqlite` path. Creation generates a UUID v4, displayed in the authentication footer. That UUID identifies this vault; it is not a secret or an authentication factor. Existing vaults unlock using that UUID and a master password. Creation leaves the new vault locked, ready for the first unlock.

The Rust storage adapter generates an independent random 32-byte vault key. Argon2id v0x13 derives a 32-byte wrapping key from the password and a random 16-byte salt. Envelope format version 1 fixes parameters at 64 MiB, three passes, and one lane. XChaCha20-Poly1305 encrypts the vault key with a fresh 24-byte nonce and binds the UUID as associated data. The database stores the format version, UUID, salt, nonce, and encrypted key in `password_envelope`. It never stores the master password. Changing parameters requires a new envelope version.

Creation prepares a private temporary database beside the destination and publishes it without replacing an existing file. Unlock validates the envelope and existing database key check before opening a session. Lock drops the repository and its key. Password strings received by the authentication handler are zeroized on completion; JavaScript and transport copies cannot guarantee memory erasure.

Password changes, recovery, and migration of earlier key-only databases are deferred. An existing database without a password envelope fails closed; it must not be treated as a new vault. Only credential passwords are encrypted by the existing storage schema; other credential fields remain plaintext.

The optional development vault still uses its disposable random-key provider. Production authentication uses `PasswordVault` and direct key-based session opening, with no OS keychain dependency. Native packaging and platform smoke tests remain separate phase 6 work.
