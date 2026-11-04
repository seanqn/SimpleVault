#include "db_local.h"
#include <QDebug>
#include <QStandardPaths>
#include <QDir>
#include <QUuid>

// groups and vault content tables could be consolidated, only content model is needed

// in production, the default connection name is databaseName param
DB_LocalStorage::DB_LocalStorage(QObject *parent, const QString &databaseName)
    : QObject(parent),
      m_connected(false),
      m_databaseName(databaseName)
{
}

DB_LocalStorage::~DB_LocalStorage() {
    if (_db.open()) {
        _db.close();
    }
}

bool DB_LocalStorage::isDBConnected() {
    return m_connected;
}

bool DB_LocalStorage::initDB() {
    // write path that Sql writes file to
    // path is hardcoded into method for consistent, valid paths across platforms
    // AppDataLocation is the standard path for persistent data

    // database name ":memory:" for testing purposes
    QString dbPath;
    if (m_databaseName == ":memory:") {
        dbPath = ":memory:";
    }
    else {
        QString appDataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
        QDir dir(appDataPath);
        if (!dir.exists() && !dir.mkpath(appDataPath)) {
            qCritical() << "Directory creation failed at " << appDataPath;
            return false;
        }
        dbPath = dir.filePath(m_databaseName + ".sqlite");

        emit directoryEstablished(appDataPath);
        emit databasePathEstablished(dbPath);
    }

    if (QSqlDatabase::contains(m_databaseName)) {
        // if database exists, connect to database
        _db = QSqlDatabase::database(m_databaseName);
    }
    else {
        // if database was not found: add, connect, open, and set database name
        _db = QSqlDatabase::addDatabase("QSQLITE", m_databaseName);
    }

    _db.setDatabaseName(dbPath);

    if (!_db.open()) {
        emit startDBConnectionFailure(_db.lastError());
        return false;
    }

    // create tables and enable foreign_keys
    QSqlQuery _query(_db);
    // _query.exec("PRAGMA foreign_keys = ON;");
    // groups table
    _query.exec("CREATE TABLE IF NOT EXISTS groups ( "
                   "id BLOB PRIMARY KEY, "
                   "name TEXT NOT NULL, "
                   "created_at DATETIME DEFAULT CURRENT_TIMESTAMP)");
    // vault_content table
    _query.exec("CREATE TABLE IF NOT EXISTS vault_content ( "
                   "group_id BLOB, "
                   "content_id BLOB PRIMARY KEY, "
                   "org_name TEXT, "
                   "username TEXT, "
                   "password TEXT, "
                   "email TEXT, "
                   "note TEXT, "
                   "FOREIGN KEY(group_id) REFERENCES groups(id) ON DELETE CASCADE) WITHOUT ROWID");

    m_connected = true;
    emit startDBConnectionSuccess();
    return true;
}

void DB_LocalStorage::closeDB() {
    _db.close();

    if (_db.isOpen()) {
        emit closeDBConnectionFailure(_db.lastError());
        return;
    }

    emit closeDBConnectionSuccess();
}

// group methods

bool DB_LocalStorage::addGroup(const Group &group) {
    if (!m_connected || !_db.isOpen()) {
        return false;
    }

    QSqlQuery _query(_db);
    _query.prepare("INSERT INTO groups (id, name) VALUES (:id, :name)");
    _query.bindValue(":id", group.id);
    _query.bindValue(":name", group.name);
    if (!_query.exec()) {
        qCritical() << "[database]: add group failed: " << _query.lastError();
        emit databaseQueryError(_query.lastError());
        return false;
    }

    emit databaseQuerySuccess();
    return true;
}

bool DB_LocalStorage::renameGroup(QByteArray groupID, const QString &newGroupName) {
    QSqlQuery _query(_db);
    _query.prepare("UPDATE groups SET name = :newName WHERE id = :groupID");
    _query.bindValue(":newName", newGroupName);
    if (!_query.exec()) {
        emit databaseQueryError(_query.lastError());
        return false;
    }

    emit databaseQuerySuccess();
    return true;
}

// _query that removes an entire group: called when a user deletes a group through qml interaction
// foreign keyed group_id in vault_content should have associated rows removed
bool DB_LocalStorage::removeGroup(QByteArray groupID) {
    QSqlQuery _query(_db);
    _query.prepare("DELETE FROM groups WHERE id = :groupID");
    _query.bindValue(":groupID", groupID);
    if(!_query.exec()) {
        emit databaseQueryError(_query.lastError());
        return false;
    }

    emit databaseQuerySuccess();
    return true;
}

// vault content (credential) methods

// saved this method
// int DB_LocalStorage::upsertVaultRowEntry(int groupID, const Credential &credential) {
//     QSqlQuery _query(_db);
//     // no longer stored as int, need to validate BLOB
//     bool insert = (credential.content_id <= 0);

//     if (insert) {
//         _query.prepare("INSERT INTO vault_content (group_id, org_name, username, password, email, note) "
//                        "VALUES (:gid, :org, :usr, :pw, :eml, :nte)");
//     }
//     else {
//         _query.prepare("UPDATE vault_content SET "
//                        "org_name = :org, "
//                        "username = :usr, "
//                        "password = :pw, "
//                        "email = :eml, "
//                        "note = :nte "
//                        "WHERE group_id = :gid AND content_id = :cid");
//         _query.bindValue(":cid", credential.content_id);
//     }

//     _query.bindValue(":gid", groupID);
//     _query.bindValue(":org", credential.org_name);
//     _query.bindValue(":usr", credential.username);
//     _query.bindValue(":pw", credential.password);
//     _query.bindValue(":eml", credential.email);
//     _query.bindValue(":nte", credential.note);

//     if (!_query.exec()) {
//         qDebug() << "db::upsertVaultRowEntry: " << _query.lastError();
//         return -1;
//     }

//     qDebug() << "db::upsertVaultRowEntry: upserted row: cid: " << _query.lastInsertId();
//     return _query.lastInsertId();
// }

bool DB_LocalStorage::upsertVaultRowEntry(QByteArray groupID, const Credential &credential) {
    QSqlQuery _query(_db);
    _query.prepare(
        "INSERT INTO vault_content (content_id, group_id, org_name, username, password, email, note) "
        "VALUES (:cid, :gid, :org, :usr, :pw, :eml, :nte) "
        "ON CONFLICT(content_id) DO UPDATE SET "
        "group_id = excluded.group_id, "
        "org_name = excluded.org_name, "
        "username = excluded.username, "
        "password = excluded.password, "
        "email = excluded.email, "
        "note = excluded.note "
        "RETURNING content_id"
    );

    _query.bindValue(":cid", credential.content_id);
    _query.bindValue(":gid", groupID);
    _query.bindValue(":org", credential.org_name);
    _query.bindValue(":usr", credential.username);
    _query.bindValue(":pw", credential.password);
    _query.bindValue(":eml", credential.email);
    _query.bindValue(":nte", credential.note);


    if (!_query.exec()) {
        qCritical() << "db::upsertVaultRowEntry Failed: " << _query.lastError();
        return false;
    }

    return true;
}