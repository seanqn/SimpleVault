import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "./components"
// import DB_LocalStorage

Window {
    id: statusBarHome
    width: 320
    height: 400
    flags: Qt.Popup | Qt.FramelessWindowHint
    visible: false
    color: "transparent"

    Connections {
        target: appStatusBar

        function onStatusItemClicked(anchorPoint, statusRect, screen) {
            if (statusBarHome.visible) {
                statusBarHome.visible = false;
            }
            else {
                if (screen) {
                    statusBarHome.screen = screen;
                }
                statusBarHome.x = anchorPoint.x - (statusBarHome.width / 2);
                statusBarHome.y = anchorPoint.y + 4;
                statusBarHome.visible = true;
                statusBarHome.raise();
                statusBarHome.requestActivate();
            }
        }
    }

    onActiveChanged: {
        if (!active) {
            visible = false;
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 12
        color: "#2B2B2B"
        border.color: "#3D3D3D"
        border.width: 1
    }
}
