
// DEPRECATED: NEW BUILD USES QTKEYCHAIN
// FOR REFERENCE ONLY

#ifndef APPLESECURITYCONTROLLER_H
#define APPLESECURITYCONTROLLER_H

#include <QObject>
#include <QString>

class VaultSecurityService : public QObject {
    Q_OBJECT

    Q_PROPERTY(QString statusText READ statusText NOTIFY statusTextChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)

public:
    explicit VaultSecurityService(QObject *parent = nullptr);
    ~VaultSecurityService();

    Q_INVOKABLE void unlockVault();

    QString statusText() const;
    bool busy() const;

public slots:
    void authRequiresRegistration();
    void authRequiresAuthentication();
    void authFailed(QString error);
    void vaultUnlocked();

signals:

    void unlockSuccessful();
    void unlockFailed();
    void statusTextChanged();
    void busyChanged();

private:
    void* objcController;

    void setBusy(bool value);
    void setStatusText(const QString &text);
    QString m_statusText;
    bool m_busy = false;
};

#endif // APPLESECURITYCONTROLLER_H
