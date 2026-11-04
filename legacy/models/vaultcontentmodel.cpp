#include "vaultcontentmodel.h"

VaultContentModel::VaultContentModel(QObject* parent, const QString &connectionName)
    : QSqlTableModel(parent, QSqlDatabase::database(connectionName))
{
    setTable("vault_content");
    setEditStrategy(QSqlTableModel::OnRowChange);

    setHeaderData(1, Qt::Horizontal, tr("org_name"));
    select();
}

QHash<int, QByteArray> VaultContentModel::roleNames() const {
    QHash<int, QByteArray> roles;
    roles[GroupIDRole] = "group_id";
    roles[ContentIDRole] = "content_id";
    roles[OrgNameRole] = "org_name";
    roles[UsernameRole] = "username";
    roles[PasswordRole] = "password";
    return roles;
}

QVariant VaultContentModel::data(const QModelIndex &index, int role) const {
    if (role < Qt::UserRole) {
        return QSqlTableModel::data(index, role);
    }

    int column;
    switch (role) {
        case GroupIDRole: column = 0; break;
        case ContentIDRole: column = 1; break;
        case OrgNameRole: column = 2; break;
        case UsernameRole: column = 3; break;
        case PasswordRole: column = 4; break;
        default: return QVariant();
    }
    return QSqlTableModel::data(this->index(index.row(), column));
}
