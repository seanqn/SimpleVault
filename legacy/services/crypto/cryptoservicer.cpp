#include "cryptoservicer.h"

CryptoServicer::CryptoServicer(QObject *parent)
    : QObject(parent) {}


/*
note that sodium is capable of clearing memory buffers for copied (toUtf8) byte arrays from the original const referenced
QString argument, but the plaintext itself is not capable of being managed by any of the existing methods as is.
if ever refactoring this service to improve security beyond service-level memory management of passed in args:
rvalue (&&) allows caller ownership to clear buffers
sodium's own sodium_malloc allow input to be read directly into non-swappable memory

the current implementation should be sufficient for debug lifecyle
*/

// encryption necessary for password storage only
std::expected<Crypto, CryptoError> CryptoServicer::encrypt(const QString &pwd, const unsigned char* key, qsizetype keySize) {
    if (key == nullptr || keySize != crypto_aead_xchacha20poly1305_ietf_KEYBYTES) {
        return std::unexpected(CryptoError::InvalidKeySize);
    }

    QByteArray pwdUtf8 = pwd.toUtf8();

    if (pwd.length() < 6 || static_cast<unsigned long long>(pwdUtf8.size()) > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX) {
        return std::unexpected(CryptoError::InvalidInput);
    }

    // define and fix size for nonce and cipher text
    unsigned char nonce[crypto_aead_xchacha20poly1305_ietf_NPUBBYTES];
    randombytes_buf(nonce, sizeof(nonce));

    QByteArray cipherText;
    cipherText.resize(pwdUtf8.size() + crypto_aead_xchacha20poly1305_ietf_ABYTES);
    unsigned long long cipherTextLen = 0;

    // call to retrieve key and assign key variable (production)

    // start encryption
    int crypt = crypto_aead_xchacha20poly1305_ietf_encrypt(
        reinterpret_cast<unsigned char*>(cipherText.data()),
        &cipherTextLen,
        reinterpret_cast<const unsigned char*>(pwdUtf8.constData()),
        static_cast<unsigned long long>(pwdUtf8.size()),
        nullptr, 0,
        nullptr,
        nonce,
        key
    );

    // clear buffer
    sodium_memzero(pwdUtf8.data(), pwdUtf8.size());

    if (crypt != 0) {
        return std::unexpected(CryptoError::EncryptionFailed);
    }

    // resize to match written bytes by encryption
    cipherText.resize(static_cast<qsizetype>(cipherTextLen));

    Crypto encryptedForm;
    encryptedForm.nonce = QByteArray(reinterpret_cast<const char*>(nonce), sizeof(nonce));
    encryptedForm.secret = cipherText;

    return encryptedForm;
}

std::expected<QString, CryptoError> CryptoServicer::decrypt(Crypto &crypt, const unsigned char* key, qsizetype keySize) {
    if (key == nullptr || keySize != crypto_aead_xchacha20poly1305_ietf_KEYBYTES) {
        return std::unexpected(CryptoError::InvalidKeySize);
    }

    if (crypt.nonce.size() != crypto_aead_xchacha20poly1305_ietf_NPUBBYTES) {
        return std::unexpected(CryptoError::InvalidInput);
    }

    if (crypt.secret.size() < crypto_aead_xchacha20poly1305_ietf_ABYTES) {
        return std::unexpected(CryptoError::InvalidInput);
    }

    // unpack and decrypt
    QByteArray pwd;
    pwd.resize(crypt.secret.size() - crypto_aead_xchacha20poly1305_ietf_ABYTES);
    unsigned long long pwdLen = 0;

    if (crypto_aead_xchacha20poly1305_ietf_decrypt(
        reinterpret_cast<unsigned char*>(pwd.data()),
        &pwdLen,
        nullptr,
        reinterpret_cast<const unsigned char*>(crypt.secret.constData()),
        static_cast<unsigned long long>(crypt.secret.size()),
        nullptr, 0,
        reinterpret_cast<const unsigned char*>(crypt.nonce.constData()),
        key) != 0) {
        sodium_memzero(pwd.data(), pwd.size());
        return std::unexpected(CryptoError::DecryptionFailed);
    }

    pwd.resize(static_cast<qsizetype>(pwdLen));
    QString result = QString::fromUtf8(pwd);
    sodium_memzero(pwd.data(), pwd.size());

    return result;
}