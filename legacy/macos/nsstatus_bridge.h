#pragma once

#include <QObject>
#include <QPoint>
#include <QRect>
#include <QScreen>
// #include <QtQml/qqmlregistration.h>

class AppStatusBar : public QObject {
    Q_OBJECT
    // QML_NAMED_ELEMENT(AppStatusBar)

public:
    explicit AppStatusBar(QObject *parent = nullptr);
    ~AppStatusBar() override;

    void setButtonIcon(const QString &resourceFPath);
    void setButtonTitle(const QString &title);

signals:
    void statusItemClicked(const QPoint &anchorPos, const QRect &buttonRect, QScreen *screen);

private:
    void *m_objcHandler{nullptr};
};
