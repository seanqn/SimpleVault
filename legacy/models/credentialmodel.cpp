#include "credentialmodel.h"
#include <QUuid>

CredentialModel::CredentialModel(QObject *parent)
    : QAbstractListModel(parent) {}

// refreshes the UI to display all credentials of the selected group (as of now is only called once after build)
void CredentialModel::update(const QList<Credential> &credentials) {
    beginResetModel();
    m_list = credentials;
    endResetModel();
}

// only appends empty rows
void CredentialModel::appendRow(const Credential &row) {
    beginInsertRows(QModelIndex(), m_list.size(), m_list.size());
    m_list.append(row);
    endInsertRows();
}

void CredentialModel::updateRow(int index, const Credential &row) {
    m_list[index] = row;
    qDebug() << "[Credential Model]::updateRow: content_id? " << QUuid::fromRfc4122(row.content_id).toString();
    QModelIndex modelIndex = createIndex(index, 0);
    // third argument left as empty vector to signify (potentially) all roles have changed
    emit dataChanged(modelIndex, modelIndex, {});
}

void CredentialModel::removeRow(int index) {
    if (index < 0 || index >= m_list.size()) {
        return;
    }

    beginRemoveRows(QModelIndex(), index, index);
    m_list.removeAt(index);
    endRemoveRows();
}

QVariant CredentialModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= m_list.size()) return QVariant();

    const Credential &credential = m_list.at(index.row());

    switch (role) {
    case ContentIDRole:
        return QUuid::fromRfc4122(credential.content_id).toString();
    case OrganizationRole:
        return credential.org_name;
    case UsernameRole:
        return credential.username;
    case PasswordRole:
        return credential.password;
    case EmailRole:
        return credential.email;
    case NoteRole:
        return credential.note;
    default:
        return QVariant();
    }
}

QHash<int, QByteArray> CredentialModel::roleNames() const {
    QHash<int, QByteArray> roles;
    roles[ContentIDRole] = "content_id";
    roles[OrganizationRole] = "org_name";
    roles[UsernameRole] = "username";
    roles[PasswordRole] = "password";
    roles[EmailRole] = "email";
    roles[NoteRole] = "note";
    return roles;
}