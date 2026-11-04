#pragma once

#include <QObject>

// used only for pushing mock rows, remove QUuid header after debugging
#include <QUuid>
#include <QDateTime>
#include "credentialtypes.h"

class DB_LocalStorage;

// Repository class communicates directly to the database, caching recent changes and handling encryption/decryption
class Repository : public QObject {
    Q_OBJECT
public:
    explicit Repository(QObject *parent = nullptr, const QString &databaseName="SimpleVault");

    void mockGroup();
    void mockCredentialRow(QByteArray groupID);

    bool initDatabase();
    bool addGroup(const Group &group);
    void fetchGroups();
    bool renameGroup(QByteArray groupID, const QString &newName);
    bool removeGroup(QByteArray groupID);

    void fetchCredentials(QByteArray groupID);
    void upsertCredentialRow(QByteArray groupID, const Credential &credential);
    void removeCredentialRow(QByteArray groupID, QByteArray contentID);

signals:
    // to models only
    void groupEntryAdded(const Group &group);
    void groupCacheUpdated(QList<Group> &cache);

    void credentialCacheUpdated(QList<Credential> &cache);
    void credentialRowRemoved(int index);

    // to controller only
    void groupCacheEmpty();
    void credentialCacheEmpty();

private:
    DB_LocalStorage *m_db;
};
