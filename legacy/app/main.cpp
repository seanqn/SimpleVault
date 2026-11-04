#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QtQml>
#include <QQmlContext>
#include "services/vaultsecurityservicer.h"
#include "controller/vaultmanager.h"
#include "data/repository.h"
#include "services/credentialeditcache.h"
#include "macos/nsstatus_bridge.h"
// includes and sub-controller dependency creation will be consolidated into a primary controller later on

int main(int argc, char *argv[])
{
    QCoreApplication::setOrganizationName("github/seanqn");
    QCoreApplication::setOrganizationDomain("com.seanqn.simplevault");
    QCoreApplication::setApplicationName("SimpleVault");

    // prevents termination upon menu-bar window closing
    QGuiApplication::setQuitOnLastWindowClosed(false);

    QGuiApplication app(argc, argv);
    QQmlApplicationEngine engine;
    AppStatusBar *statusBar = new AppStatusBar(&app);
    engine.rootContext()->setContextProperty("appStatusBar", statusBar);

    bool runInTest = true;
    int testCase = 2;

    Repository mainRepository(nullptr, runInTest ? ":memory:" : "SimpleVault");
    CredentialEditCache editCache;
    VaultManager *vaultManager = new VaultManager(mainRepository, editCache, &app);

    qmlRegisterSingletonInstance("VaultManager", 1, 0, "VaultManager", vaultManager);

    // authentication servicer
    VaultSecurityServicer *vaultAuthenticator = new VaultSecurityServicer(&app);
    qmlRegisterSingletonInstance("VaultSecurityServicer", 1, 0, "Authenticator", vaultAuthenticator);

    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(-1); },
        Qt::QueuedConnection
    );

    if (testCase == 1) {
        engine.load(QUrl(QStringLiteral("qrc:/qt/qml/SimpleVault/tests/groups_vaultcontent_tests.qml")));
    }
    else if (testCase == 2) {
        engine.load(QUrl(QStringLiteral("qrc:/qt/qml/SimpleVault/views/home/Home.qml")));
    }
    else if (testCase == 3) {
        engine.load(QUrl(QStringLiteral("qrc:/qt/qml/SimpleVault/views/Main.qml")));
    }
    else {
        return 0;
    }

    // engine.rootContext()->setContextProperty("vaultManager", vaultManager);
    return app.exec();
}
