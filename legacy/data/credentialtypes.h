#pragma once
#include <QDateTime>

struct Group {
public:
    QByteArray id;
    QString name;
    QDateTime created_at;
};

struct Credential {
public:
    // now storing content ids as (credentialtype: QByteArray=QUuid, database: BLOB) for safer id creation and keying
    // creation is at VaultManager::addDefaultCredentialRow
    QByteArray content_id;
    QString org_name;
    QString username;
    QString password;
    QString email;
    QString note;
};
