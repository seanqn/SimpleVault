//! Legacy-compatible XChaCha20-Poly1305 with empty associated data.
use chacha20poly1305::{
    aead::{rand_core::RngCore, Aead, OsRng},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use zeroize::Zeroizing;

pub struct SecretKey(Zeroizing<[u8; 32]>);
impl SecretKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        Ok(Self(Zeroizing::new(
            bytes.try_into().map_err(|_| CryptoError::InvalidKeySize)?,
        )))
    }
}
/// Providers must return an owned key or an error when the vault is locked.
/// OS keychain integration is implemented by the desktop adapter in a later phase.
pub trait KeyProvider {
    type Error;
    fn load_key(&self) -> Result<SecretKey, Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub enum CryptoError {
    InvalidKeySize,
    InvalidInput,
    RandomnessUnavailable,
    EncryptionFailed,
    DecryptionFailed,
}
#[derive(Clone)]
pub struct EncryptedCredential {
    pub nonce: [u8; 24],
    pub secret: Vec<u8>,
}

/// Takes ownership so the service's plaintext allocation is erased on every exit.
/// Caller copies and the legacy edit cache are outside this guarantee.
pub fn encrypt(
    password: Zeroizing<String>,
    key: &SecretKey,
) -> Result<EncryptedCredential, CryptoError> {
    if password.encode_utf16().count() < 6 {
        return Err(CryptoError::InvalidInput);
    }
    encrypt_stored_password(password, key)
}

/// Encrypt an existing stored value, including empty or short legacy passwords.
/// Password-strength policy belongs to the application, not data migration.
pub fn encrypt_stored_password(
    password: Zeroizing<String>,
    key: &SecretKey,
) -> Result<EncryptedCredential, CryptoError> {
    let mut nonce = [0; 24];
    OsRng
        .try_fill_bytes(&mut nonce)
        .map_err(|_| CryptoError::RandomnessUnavailable)?;
    seal(password.as_bytes(), key, nonce)
}
fn seal(
    plaintext: &[u8],
    key: &SecretKey,
    nonce: [u8; 24],
) -> Result<EncryptedCredential, CryptoError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key.0.as_ref())
        .map_err(|_| CryptoError::InvalidKeySize)?;
    let secret = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    Ok(EncryptedCredential { nonce, secret })
}
pub fn decrypt(
    encrypted: &EncryptedCredential,
    key: &SecretKey,
) -> Result<Zeroizing<String>, CryptoError> {
    if encrypted.secret.len() < 16 {
        return Err(CryptoError::InvalidInput);
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key.0.as_ref())
        .map_err(|_| CryptoError::InvalidKeySize)?;
    let bytes = Zeroizing::new(
        cipher
            .decrypt(
                XNonce::from_slice(&encrypted.nonce),
                encrypted.secret.as_ref(),
            )
            .map_err(|_| CryptoError::DecryptionFailed)?,
    );
    let text = std::str::from_utf8(&bytes).map_err(|_| CryptoError::InvalidInput)?;
    Ok(Zeroizing::new(text.to_owned()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn libsodium_fixture() {
        let key = SecretKey::from_bytes(&[7; 32]).unwrap();
        let expected = include_bytes!("fixture.bin");
        let sealed = seal("secret🔐".as_bytes(), &key, [9; 24]).unwrap();
        assert_eq!(sealed.secret, expected);
        assert_eq!(&**decrypt(&sealed, &key).unwrap(), "secret🔐");
    }
    #[test]
    fn rejects_tampering_and_wrong_keys() {
        let key = SecretKey::from_bytes(&[7; 32]).unwrap();
        let original = encrypt(Zeroizing::new("secret🔐".into()), &key).unwrap();
        let wrong = SecretKey::from_bytes(&[8; 32]).unwrap();
        assert!(matches!(
            decrypt(&original, &wrong),
            Err(CryptoError::DecryptionFailed)
        ));
        for i in 0..original.secret.len() {
            let mut altered = original.clone();
            altered.secret[i] ^= 1;
            assert!(matches!(
                decrypt(&altered, &key),
                Err(CryptoError::DecryptionFailed)
            ));
        }
        let mut altered = original.clone();
        altered.nonce[0] ^= 1;
        assert!(decrypt(&altered, &key).is_err());
        altered.secret.clear();
        assert!(matches!(
            decrypt(&altered, &key),
            Err(CryptoError::InvalidInput)
        ));
        assert_ne!(
            original.nonce,
            encrypt(Zeroizing::new("secret🔐".into()), &key)
                .unwrap()
                .nonce
        );
    }
    #[test]
    fn legacy_length_and_key_validation() {
        assert!(matches!(
            SecretKey::from_bytes(&[0; 31]),
            Err(CryptoError::InvalidKeySize)
        ));
        let key = SecretKey::from_bytes(&[0; 32]).unwrap();
        for text in ["", "12345"] {
            assert!(matches!(
                encrypt(Zeroizing::new(text.into()), &key),
                Err(CryptoError::InvalidInput)
            ));
        }
        assert!(encrypt(Zeroizing::new("🔐🔐🔐".into()), &key).is_ok());
    }
}
