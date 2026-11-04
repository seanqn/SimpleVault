#include <QDebug>
#include <QUuid>
#include "vaultsecurityservicer.h"

VaultSecurityServicer::VaultSecurityServicer(QObject *parent)
    : QObject(parent),
      m_appSettings("seanqn", "SimpleVault")
{
    m_uid = m_appSettings.value("uid").toString();

    // set false for testing, will probably want keep authentication status throughout app builds around specific conditions
    m_authenticated = false;
    qDebug() << "VaultSecurityServicer started. UID: " << (m_uid.isEmpty() ? "None" : m_uid);
}

bool VaultSecurityServicer::isRegistered() {
    return !m_appSettings.value("uid").toString().isEmpty();
}

bool VaultSecurityServicer::isAuthenticated() {
    return m_authenticated;
}

void VaultSecurityServicer::registerUser(const QString password) {
    // QSettings settings("seanqn", "SimpleVault");
    // m_uid = settings.value("uid").toString();
    if (!m_uid.isEmpty()) {
        qDebug() << "User already registered. UID " << m_uid;
        emit error("UID already registered");
        return;
    }

    // QSettings settings("seanqn", "SimpleVault");
    m_uid = QUuid::createUuid().toString(QUuid::WithoutBraces);

    m_appSettings.setValue("uid", m_uid);
    m_appSettings.sync();

    qDebug() << "UID set to: " << m_uid;

    auto *job = new QKeychain::WritePasswordJob(QLatin1String("com.seanqn.simplevault"), this);
    job->setKey(m_uid);
    job->setBinaryData(password.toUtf8());

    connect(job, &QKeychain::WritePasswordJob::finished, [this, job]() {
        job->deleteLater();
        qDebug() << job->errorString();
        if (job->error()) {
            qDebug() << "Keychain error: " << job->errorString();
            emit error(
                tr("Write failed: %1").arg(job->errorString()));
        }
        else {
            emit registrationSuccessful();
            // emit keyStored(m_uid);
        }
    });

    job->start();

    emit updateUID();

    // for testing read capability/persistency across builds
    // currently reads password from both textData() and binaryData()
    qDebug() << "Performing keychain read from same registration...";
    auto *readJob = new QKeychain::ReadPasswordJob(QLatin1String("com.seanqn.simplevault"), this);
    readJob->setKey(m_uid);
    connect(readJob, &QKeychain::ReadPasswordJob::finished, [this, readJob]() {
        if (!readJob->error()) {
            qDebug() << "UID retrieved from Keychain: " << readJob->textData();
            qDebug() << "Password retrieved from Keychain: " << readJob->binaryData();
        }
    });

    readJob->start();
}

void VaultSecurityServicer::authenticateUser(const QString &password) {
    if (m_uid.isEmpty()) {
        emit error("No user registered");
        return;
    }

    auto *job = new QKeychain::ReadPasswordJob(QLatin1String("com.seanqn.simplevault"), this);
    job->setKey(m_uid);

    connect(job, &QKeychain::ReadPasswordJob::finished, [this, job, password]() {
        job->deleteLater();
        if (job->error()) {
            qDebug() << "Keychain error: " << job->errorString();
            emit error(
                tr("Read failed: %1").arg(job->errorString()));
            return;
        }

        QString retrievedPassword = QString::fromUtf8(job->binaryData());

        if (retrievedPassword == password) {
            m_authenticated = true;
            emit authSuccessful();
            emit isAuthenticatedChanged();
        }
    });

    job->start();
}

// unimplemented, requires testing
void VaultSecurityServicer::deleteUser() {
    auto *job = new QKeychain::DeletePasswordJob(QLatin1String("com.seanqn.simplevault"), this);
    job->setKey(m_uid);

    QObject::connect(job, &QKeychain::DeletePasswordJob::finished, [this, job]() {
        job->deleteLater();
        if (job->error()) {
            qDebug() << "Keychain error: " << job->errorString();
            emit error(
                tr("Delete failed: %1").arg(job->errorString()));
            return;
        }

        if (m_uid.isEmpty()) {
            emit deleteSuccessful();
        }
    });

    job->start();
}

// OUTDATED METHODS

// void VaultSecurityServicer::readKey(const QString &key) {
//     auto *job = new QKeychain::WritePasswordJob(QLatin1String("com.seanqn.simplevault"), this);
//     job->setKey(key);
//     m_readCredential.setKey(key);

//     QObject::connect(&m_readCredential, &QKeychain::ReadPasswordJob::finished, [=]() {
//         if (m_readCredential.error()) {
//             emit error(
//                 tr("Process failed reading key: %1").arg(qPrintable(m_readCredential.errorString())));
//             return;
//         }
//         emit keyRestored(key, m_readCredential.textData());
//     });

//     m_readCredential.start();
// }


// void VaultSecurityServicer::writeKey(const QString &key, const QString &value) {
//     m_writeCredential.setKey(key);

//     QObject::connect(&m_writeCredential, &QKeychain::WritePasswordJob::finished, [=]() {
//         if (m_writeCredential.error()) {
//             emit error(
//                 tr("Process failed writing key: %1").arg(qPrintable(m_readCredential.errorString())));
//             return;
//         }
//         emit keyStored(key);
//     });

//     m_writeCredential.setTextData(value);
//     m_writeCredential.start();
// }

// void VaultSecurityServicer::deleteKey(const QString &key) {
//     m_deleteCredential.setKey(key);

//     QObject::connect(&m_deleteCredential, &QKeychain::DeletePasswordJob::finished, [=]() {
//         if (m_deleteCredential.error()) {
//             emit error(
//                 tr("Process failed deleting key: %1").arg(qPrintable(m_deleteCredential.errorString())));
//             return;
//         }
//         emit keyDeleted(key);
//     });

//     m_deleteCredential.start();
// }

// void VaultSecurityServicer::unlockVault() {
//     if (!vaultKey) {
//         self->setBusy(true);
//         setStatusText("Registration required...");

//         self->registratePasskey();
//     }
//     else {
//         self->setBusy(true);
//         setStatusText("Authenticating...");

//         self->authenticatePasskey();
//     }
// }

// void VaultSecurityServicer::registratePasskey() {

// }

// void VaultSecurityServicer::authenticatePasskey() {

// }

//signals
// void VaultSecurityServicer::setStatusText(const QString &text) {
//     if (_statusText == text) {
//         return;
//     }

//     _statusText = text;
//     emit statusTextChanged();
// }

// QString VaultSecurityServicer::statusText() const {
//     return _statusText;
// }

// void VaultSecurityServicer::setBusy(bool value) {
//     if (_busy == value) {
//         return;
//     }

//     _busy = value;
//     emit busyChanged();
// }

// bool VaultSecurityServicer::busy() const {
//     return _busy;
// }

//slots
