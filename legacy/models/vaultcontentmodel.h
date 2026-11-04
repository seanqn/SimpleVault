#pragma once
#include <QSqlTableModel>

class VaultContentModel : public QSqlTableModel {
    Q_OBJECT

public:
    enum ContentRoles {
        GroupIDRole = Qt::UserRole + 1,
        ContentIDRole,
        OrgNameRole,
        UsernameRole,
        PasswordRole
    };
    explicit VaultContentModel(QObject* parent = nullptr, const QString &connectionName="SimpleVault");
    virtual ~VaultContentModel() {}
    QHash<int, QByteArray> roleNames() const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
};
