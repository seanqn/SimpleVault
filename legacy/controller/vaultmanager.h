#pragma once

#include <QObject>
#include <QUuid>
#include <QDateTime>
// property objects (CredentialModel, GroupsModel) require full class definitions
#include "models/credentialmodel.h"
#include "models/groupsmodel.h"

class Repository;
class CredentialEditCache;

class VaultManager : public QObject {
    Q_OBJECT
    Q_PROPERTY(CredentialModel* credentialModel READ credentialModel CONSTANT)
    Q_PROPERTY(GroupsModel* groupsModel READ groupsModel CONSTANT)

    Q_PROPERTY(bool isGroupSelected READ isGroupSelected NOTIFY groupChanged)
    Q_PROPERTY(QString getCurrentGroupID READ getCurrentGroupID NOTIFY groupChanged)
    Q_PROPERTY(QString getCurrentContentID READ getCurrentContentID)

public:
    explicit VaultManager(Repository& vaultRepository, CredentialEditCache& editCache, QObject* parent = nullptr);
    ~VaultManager();
    GroupsModel* groupsModel() const { return m_groupsModel; }
    CredentialModel* credentialModel() const { return m_credentialModel; }
    void initRepository();
    // void updateGroups();

    // group services (GroupsModel)
    Q_INVOKABLE void createGroup(const QString &name);
    Q_INVOKABLE void selectGroup(int index);
    Q_INVOKABLE void renameGroup(int index, const QString &newName);
    Q_INVOKABLE void removeGroup(int index);
    bool isGroupSelected() { return !m_currentGroupID.isNull(); }
    Q_INVOKABLE QString getCurrentGroupID() const { return QUuid::fromRfc4122(m_currentGroupID).toString(); }

    // credential content services (CredentialModel)
    Q_INVOKABLE void addDefaultCredentialRow();
    Q_INVOKABLE void selectCredentialRow(int rowIndex);
    // Q_INVOKABLE void removeCredentialRow();
    Q_INVOKABLE QString getCurrentContentID() const { return QUuid::fromRfc4122(m_currentContentID).toString(); }

    // edit cache service
    Q_INVOKABLE void startRowEdit(int rowIndex);
    Q_INVOKABLE void updateEditCache(const QString &role, const QString &value);
    Q_INVOKABLE void submitEditCache();

signals:
    void repositoryInitializationError();
    void groupAdded();
    void createGroupError();
    void groupChanged(const QString &id);
    void groupRenamed(const QString &name);
    void groupRenameError();
    void groupRemoved(const QString &id);
    void groupRemoveError(const QString &id);

private:
    GroupsModel* m_groupsModel;
    CredentialModel* m_credentialModel;
    Repository& m_repo;
    CredentialEditCache& m_editCache;

    QByteArray m_currentGroupID;
    QByteArray m_currentContentID;

    int m_editRowIndex = -1;
};
