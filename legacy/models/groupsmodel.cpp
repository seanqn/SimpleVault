#include "groupsmodel.h"
#include <QUuid>

// data() was updated to return Group struct members

GroupsModel::GroupsModel(QObject *parent)
    : QAbstractListModel(parent) {}

QByteArray GroupsModel::getGroupIDAt(int rowIndex) {
    if (rowIndex < 0 || rowIndex >= m_list.size()) {
        return QByteArray();
    }

    return m_list[rowIndex].id;
}

// update() is when the model needs to be populated with the groups database table during builds
void GroupsModel::update(const QList<Group> &groups) {
    beginResetModel();
    m_list = groups;
    endResetModel();
}

void GroupsModel::append(const Group &group) {
    beginInsertRows(QModelIndex(), m_list.size(), m_list.size());
    m_list.append(group);
    endInsertRows();
}

// replacement is only relevant to a groups Group.name member
// TODO: update rename method to accept index for instant lookup
void GroupsModel::rename(int index, const QString &name) {
    if (index < 0 || index >= m_list.size()) {
        return;
    }

    m_list[index].name = name;
    QModelIndex indx = createIndex(index, 0);
    emit dataChanged(indx, indx, {GroupNameRole});
}

void GroupsModel::remove(int index) {
    // index could not be out of range, standard guard
    if (index < 0 || index >= m_list.size()) {
        return;
    }

    beginRemoveRows(QModelIndex(), index, index);
    m_list.removeAt(index);
    endRemoveRows();
}

QHash<int, QByteArray> GroupsModel::roleNames() const {
    QHash<int, QByteArray> roles;
    roles[IDRole] = "id";
    roles[GroupNameRole] = "name";
    roles[CreatedAtRole] = "created_at";
    return roles;
}

QVariant GroupsModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= m_list.size()) return QVariant();

    const Group &group = m_list.at(index.row());

    switch (role) {
    case IDRole:
        return QUuid::fromRfc4122(group.id).toString();
    case GroupNameRole:
        return group.name;
    case CreatedAtRole:
        return group.created_at;
    default:
        return QVariant();
    }
}

