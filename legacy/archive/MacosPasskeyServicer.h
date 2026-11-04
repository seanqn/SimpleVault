// PasskeyDelegate leverages AuthenticationServices' "delegates" aka callback handlers
// Some API functionalities like delegates are restricted to ObjC and require bridging

#ifndef MACOSPASSKEYSERVICER_H
#define MACOSPASSKEYSERVICER_H
#include <QObject>

class MacosPasskeyServicer : public QObject {
    Q_OBJECT
public:
    explicit MacosPasskeyServicer(QObject *parent = nullptr);
    ~MacosPasskeyServicer();

signals:
    void authSuccessful(const QString &token);
    void authFailed(const QString &error);

public slots:
    void registerPasskey();
    void authenticateWithPasskey();

private:
    void* objcBridge;
};

#endif
