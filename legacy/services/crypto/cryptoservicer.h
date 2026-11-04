// TODO: Finish declarations

#pragma once

#include <QObject>
#include <expected>
#include "cryptotypes.h"
#include <sodium.h>

// standard library include: note that expectation results (like encrypt() types) need to be registered as a meta type if queueing signals/slots
// qRegisterMetaType<expected<types>>(): for queued connections (signal emitted in background)

class CryptoServicer : public QObject {
    Q_OBJECT
public:
    explicit CryptoServicer(QObject *parent = nullptr);
    // ~CryptoServicer();

    std::expected<Crypto, CryptoError> encrypt(const QString &pwd, const unsigned char *key, qsizetype keySize);
    std::expected<QString, CryptoError> decrypt(Crypto &crypt, const unsigned char *key, qsizetype keySize);

public slots:

signals:

private:
    QString key;
};
