#include "repository.h"
#include "repositories/db_local.h"

Repository::Repository(QObject *parent, const QString &databaseName)
    : QObject(parent)
{
    m_db = new DB_LocalStorage(this, databaseName);
}

// emitted repository signals have listening slots in the model classes, and is how the models are being updated

bool Repository::initDatabase() {
    return m_db->initDB();
}

void Repository::mockGroup() {
    Group mockGroup;
    mockGroup.id = QUuid::createUuidV7().toRfc4122();
    mockGroup.name = "mock_group_1";
    if (m_db->addGroup(mockGroup)) {
        mockCredentialRow(mockGroup.id);
        return;
    }
}

// adds mock credential row to test fetching
void Repository::mockCredentialRow(QByteArray groupID) {
    // group id, organization, username, pasword, email, additional optional notes
    Credential mockRow;
    mockRow.content_id = QUuid::createUuidV7().toRfc4122();
    qDebug() << "[repository]: calling mockCredentialRow for group " << groupID << ", content id: " << QUuid::fromRfc4122(mockRow.content_id).toString();
    mockRow.org_name = "mock_org";
    mockRow.username = "mock_user";
    mockRow.password = "mock_pass";
    mockRow.email = "mock_email";
    mockRow.note = "mock_note";
    upsertCredentialRow(groupID, mockRow);
}

bool Repository::addGroup(const Group &group) {
    if (m_db->addGroup(group)) {
        qDebug() << "[repository]: new group struct: [id]: " << group.id << " [name]: " << group.name << "[created_at]: " << group.created_at;
        emit groupEntryAdded(group);
        return true;
    }

    return false;
}

// the only method where the cache is implemented is for the purpose of overwriting the group model's internal list
void Repository::fetchGroups() {
    qDebug() << "[repository]: fetching groups";
    QList<Group> groupCache = m_db->fetchRecords<Group>(
        "groups",
        {"id", "name", "created_at"},
        [](auto mapper) {
            Group g;
            g.id = mapper("id").toByteArray();
            g.name = mapper("name").toString();
            g.created_at = mapper("created_at").toDateTime();
            return g;
        }
    );

    emit groupCacheUpdated(groupCache);

    if (groupCache.isEmpty()) {
        qDebug() << "[repository]: group cache was empty";
    }
    else {
        qDebug() << "[repository]: group cache was updated with the items from database";
    }
}

bool Repository::renameGroup(QByteArray groupID, const QString &newName) {
    if (!(m_db->renameGroup(groupID, newName))) {
        return false;
    }

    return true;
}

// based on the foreign key groups id = vault_content group_id, removal of a group is expected to cascade to all associated credentials
// requires a return that is passed along to the controller to update the groups model
bool Repository::removeGroup(QByteArray groupID) {
    if (m_db->removeGroup(groupID)) {
        return true;
    }

    return false;
}

// lambda effectively passes a template type Mapping object to the required paramater
void Repository::fetchCredentials(QByteArray groupID) {
    qDebug() << "[repository]: fetching credentials";
    QList<Credential> credentialCache = m_db->fetchRecords<Credential>(
        "vault_content",
        {"content_id", "org_name", "username", "password", "email", "note"},
        [](auto mapper) {
            Credential c;
            c.content_id = mapper("content_id").toByteArray();
            c.org_name = mapper("org_name").toString();
            c.username = mapper("username").toString();
            c.password = mapper("password").toString();
            c.email = mapper("email").toString();
            c.note = mapper("note").toString();
            return c;
        },
        "WHERE group_id = :groupID",
        {{":groupID", groupID}}
    );

    emit credentialCacheUpdated(credentialCache);

    if (credentialCache.isEmpty()) {
        qDebug() << "[repository]: credential cache was empty";
    }
    else {
        qDebug() << "[repository]: credential cache was updated with the items from database";
    }
}

void Repository::upsertCredentialRow(QByteArray groupID, const Credential &credential) {
    if (groupID.isNull()) {
        qDebug() << "[repository]: upsertCredentialRow: groupID is null";
        return;
    }

    bool upsertedRowContentID = m_db->upsertVaultRowEntry(groupID, credential);

    if (!upsertedRowContentID) {
        qDebug() << "[repository]: credential row could not be upserted.";
        return;
    }

    qDebug() << "[repository]: credential row successfully upserted";
}

// called when an entire entry is removed
void Repository::removeCredentialRow(QByteArray groupID, QByteArray contentID) {

}