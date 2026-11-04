#ifndef MACOSKEYCHAINSERVICER_H
#define MACOSKEYCHAINSERVICER_H

// header enables ObjC++ file for compatibility with Macos keychain services in Security : Apple SDK

#include <QObject>

class MacosKeychainServicer : public QObject {
    Q_OBJECT
public:
    explicit MacosKeychainServicer(QObject *parent = nullptr);

signals:

public slots:
    void generateAESKey();

    // may be a redundant method if corresponding passkey servicer methods
    // will verify that the class member exists
    void validateAESKey();
    void _addAESKey();
    // void _fetchSecItem();

private:
    void* _AESKey;
};

#endif // MACOSKEYCHAINSERVICER_H
