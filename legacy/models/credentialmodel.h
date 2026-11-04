#pragma once

#include <QAbstractListModel>
#include "data/credentialtypes.h"

class CredentialModel : public QAbstractListModel {
    Q_OBJECT

public:
    explicit CredentialModel(QObject *parent = nullptr);
    enum CredentialRoles {
        ContentIDRole,
        OrganizationRole,
        UsernameRole,
        PasswordRole,
        EmailRole,
        NoteRole
    };
    QHash<int, QByteArray> roleNames() const override;
    QVariant data(const QModelIndex &index, int role) const override;
    int rowCount(const QModelIndex &parent = QModelIndex()) const override { return m_list.size(); }
    // columnCount expects reference to a parent but it is unused, no argument required
    int columnCount(const QModelIndex&) const override { return 5; }
    Credential getCredentialAt(int rowIndex) { return m_list[rowIndex]; }

public slots:
    void update(const QList<Credential> &credentials);
    void appendRow(const Credential &row = Credential{});
    // void syncNewRow(const Credential &row);
    void updateRow(int index, const Credential &row);
    void removeRow(int index);

private:
    QList<Credential> m_list;
};
