#pragma once

struct Crypto {
public:
    QByteArray nonce;
    QByteArray secret;
};

enum class CryptoError {
    InvalidKeySize,
    InvalidInput,
    EncryptionFailed,
    DecryptionFailed
};
