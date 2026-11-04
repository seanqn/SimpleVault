#pragma once

#include <QAbstractListModel>
#include "data/credentialtypes.h"

class GroupsModel : public QAbstractListModel {
    Q_OBJECT

public:
    explicit GroupsModel(QObject *parent = nullptr);
    enum GroupRoles {
        IDRole,
        GroupNameRole,
        CreatedAtRole
    };
    QHash<int, QByteArray> roleNames() const override;
    QVariant data(const QModelIndex &index, int role) const override;
    int rowCount(const QModelIndex &parent = QModelIndex()) const override { return m_list.size(); }
    QByteArray getGroupIDAt(int rowIndex);

public slots:
    void update(const QList<Group> &groups);
    void append(const Group &group);
    void rename(int index, const QString &name);
    void remove(int index);

private:
    // the actual structure of this model essentially just serves as the destination for the repository group cache
    QList<Group> m_list;
};
