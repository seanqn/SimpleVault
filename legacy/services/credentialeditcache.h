#pragma once

#include <QObject>
#include "data/credentialtypes.h"

class CredentialEditCache : public QObject
{
    Q_OBJECT
public:
    explicit CredentialEditCache(QObject *parent = nullptr);

    bool isEditActive() { return editActiveState; }
    void setDefaultRowState(bool state) { defaultRowNotSubmitted = state; }
    bool defaultRowExists() { return defaultRowNotSubmitted; }
    bool editCacheIsEmpty();
    bool editCacheIsClean();

    void startEdit(Credential &cred);
    void updateColumn(const QString &role, const QString &value);
    Credential getCache() const { return m_edit; }
    void reset();


private:
    bool defaultRowNotSubmitted = false;
    bool editActiveState = false;
    Credential m_row;
    Credential m_edit;
};
