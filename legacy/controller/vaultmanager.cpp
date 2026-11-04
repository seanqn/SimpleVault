#include "vaultmanager.h"
#include "data/repository.h"
#include "services/credentialeditcache.h"

VaultManager::VaultManager(Repository& vaultRepository, CredentialEditCache& editCache, QObject *parent)
    : QObject(parent),
    m_repo(vaultRepository),
    m_editCache(editCache)
{

    m_groupsModel = new GroupsModel(this);
    connect(&m_repo, &Repository::groupCacheUpdated, m_groupsModel, &GroupsModel::update);
    connect(&m_repo, &Repository::groupEntryAdded, m_groupsModel, &GroupsModel::append);

    m_credentialModel = new CredentialModel(this);
    connect(&m_repo, &Repository::credentialCacheUpdated, m_credentialModel, &CredentialModel::update);

    initRepository();
}

VaultManager::~VaultManager() {
    if (m_editRowIndex != -1) {
        submitEditCache();
    }
}

void VaultManager::initRepository() {
    if (!m_repo.initDatabase())  {
        emit repositoryInitializationError();
        return;
    }
    m_repo.mockGroup();
    m_repo.fetchGroups();
}

/*
groups model management methods
*/

// group object instantiation lives here for now
void VaultManager::createGroup(const QString &name) {
    Group newGroup;
    newGroup.id = QUuid::createUuidV7().toRfc4122();
    newGroup.name = name;
    newGroup.created_at = QDateTime::currentDateTime();
    if (m_repo.addGroup(newGroup)) {
        return;
    }

    emit createGroupError();
}

void VaultManager::selectGroup(int index) {
    // ensure that the edit cache is reset before group switching
    if (m_editCache.isEditActive()) {
        submitEditCache();
    }

    QByteArray gid = m_groupsModel->getGroupIDAt(index);
    if (gid.isEmpty()) {
        qDebug() << "[Vault Manager]: selectGroup: group list index out of range";
        return;
    }
    if (m_currentGroupID == gid) return;

    m_repo.fetchCredentials(gid);
    m_currentGroupID = gid;
    emit groupChanged(QUuid::fromRfc4122(m_currentGroupID).toString());
}

void VaultManager::renameGroup(int index, const QString &newName) {
    QByteArray gid = m_groupsModel->getGroupIDAt(index);
    if (gid.isEmpty()) {
        qDebug() << "[Vault Manager]: renameGroup: group list index out of range";
        return;
    }

    if (m_repo.renameGroup(gid, newName)) {
        m_groupsModel->rename(index, newName);
        emit groupRenamed(newName);
        return;
    }

    emit groupRenameError();
}

void VaultManager::removeGroup(int index) {
    QByteArray gid = m_groupsModel->getGroupIDAt(index);
    if (gid.isEmpty()) {
        qDebug() << "[Vault Manager]: removeGroup: group list index out of range";
        return;
    }

    if (m_repo.removeGroup(gid)) {
        m_groupsModel->remove(index);
        emit groupRemoved(QUuid::fromRfc4122(gid).toString());
        m_currentGroupID.clear();
        return;
    }

    emit groupRemoveError(QUuid::fromRfc4122(gid).toString());
}

/*
credential model management methods
*/

// credential object instantation lives here for now
void VaultManager::addDefaultCredentialRow() {
    // TODO: allow new rows as long as text for one column in the previous default row has been changed
    if (m_editCache.defaultRowExists()) {
        return;
    }

    if (m_editCache.isEditActive()) {
        submitEditCache();
    }

    Credential newRow{};

    // new row key is created here as a 16 byte time-based uuid (must be 16 bytes to read as BLOB in db without conversion)
    newRow.content_id = QUuid::createUuidV7().toRfc4122();
    m_credentialModel->appendRow(newRow);
}

void VaultManager::selectCredentialRow(int rowIndex) {
    qDebug() << "[VaultManager]: selectCredentialRow called to select row with model index: " << rowIndex;
}

void VaultManager::startRowEdit(int rowIndex) {
    int rowCount = m_credentialModel->rowCount();
    if (rowIndex < 0 || rowIndex >= rowCount) return;
    Credential cred = m_credentialModel->getCredentialAt(rowIndex);

    if (m_editRowIndex != -1) {
        submitEditCache();
    }

    m_editRowIndex = rowIndex;
    m_editCache.startEdit(cred);
}

void VaultManager::updateEditCache(const QString &role, const QString &value) {
    m_editCache.updateColumn(role, value);
}

// called when the edit index has changed or active focus loss from editing row
void VaultManager::submitEditCache() {
    if (m_editCache.editCacheIsEmpty()) {
        qDebug() << "[VaultManager]: edit cache was empty";
        m_credentialModel->removeRow(m_editRowIndex);
        return;
    }

    if (m_editCache.editCacheIsClean()) {
        qDebug () << "[Vault Manager]: edit cache was clean";
        return;
    }

    m_repo.upsertCredentialRow(m_currentGroupID, m_editCache.getCache());
    m_editCache.reset();
    m_editRowIndex = -1;
}
