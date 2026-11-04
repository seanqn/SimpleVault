#include "credentialeditcache.h"

CredentialEditCache::CredentialEditCache(QObject *parent)
    : QObject(parent) {}

bool CredentialEditCache::editCacheIsEmpty() {
    bool isEmpty = m_edit.org_name.isEmpty() &&
                   m_edit.username.isEmpty() &&
                   m_edit.password.isEmpty() &&
                   m_edit.email.isEmpty() &&
                   m_edit.note.isEmpty();
    return isEmpty;
}

bool CredentialEditCache::editCacheIsClean() {
    bool isClean = (m_edit.org_name == m_row.org_name &&
                    m_edit.username == m_row.username &&
                    m_edit.password == m_row.password &&
                    m_edit.email == m_row.email &&
                    m_edit.note == m_row.note);
    return isClean;
}

void CredentialEditCache::startEdit(Credential &cred) {
    editActiveState = true;
    m_row = cred;
    m_edit = cred;
}

void CredentialEditCache::updateColumn(const QString &role, const QString &value) {
    if (!isEditActive()) return;

    if (role == "org_name") {
        m_edit.org_name = value;
    }
    else if (role == "username") {
        m_edit.username = value;
    }
    else if (role == "password") {
        m_edit.password = value;
    }
    else if (role == "email") {
        m_edit.email = value;
    }
    else if (role == "note") {
        m_edit.note = value;
    }
    else {
        return;
    }
}

void CredentialEditCache::reset() {
    editActiveState = false;
    m_row = Credential{};
    m_edit = Credential{};
}