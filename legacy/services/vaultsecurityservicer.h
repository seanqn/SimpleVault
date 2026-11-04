#pragma once

#include <QObject>
#include <QSettings>

// cmake will call fetch on git repo qtkeychain
// relative file path: qtkeychain/keychain.h ?
#include "qtkeychain/keychain.h"

// include changed, verify class members

class VaultSecurityServicer : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString uid MEMBER m_uid NOTIFY sendUID)
    Q_PROPERTY(bool isRegistered READ isRegistered NOTIFY isRegisteredChanged)
    Q_PROPERTY(bool isAuthenticated READ isAuthenticated NOTIFY isAuthenticatedChanged)
public:
    explicit VaultSecurityServicer(QObject *parent = nullptr);
    // ~VaultSecurityServicer();

    Q_INVOKABLE void registerUser(const QString password);
    Q_INVOKABLE void authenticateUser(const QString &password);
    Q_INVOKABLE void deleteUser();
    Q_INVOKABLE QString updateUID() { return m_uid; }
    bool isRegistered();
    bool isAuthenticated();

Q_SIGNALS:
    // void registratePasskey();
    // void authenticatePasskey();

    // QString statusText() const;
//     bool busy() const;

// public slots:
    // void authRequiresRegistration();
//     void authRequiresAuthentication();
//     void authFailed(QString error);
//     void vaultUnlocked();

signals:
    // first three signals have too similar purposes to key signals

    void registrationSuccessful();
    void authSuccessful();
    void deleteSuccessful();
    // void keyStored(const QString &key);
    // void keyRestored(const QString &key, const QString &value);
    // void keyDeleted(const QString &key);
    void error(const QString &errorText);
    void sendUID(const QString &uid);
    void isRegisteredChanged();
    void isAuthenticatedChanged();

    // void statusTextChanged();
//     void busyChanged();

private:
    QString m_uid;
    QSettings m_appSettings;
    bool m_authenticated;

    // bool setBusy(bool value) const;
    // void setStatusText(const QString &text);
    // QString _statusText;
    // bool _busy = false;

};
