import QtQuick 2.15
import QtQuick.Controls
import QtQuick.Layouts
import VaultSecurityServicer

ApplicationWindow {
    id: mainWindow
    visible: true
    title: "SimpleVault"
    width: 700
    height: 600
    color: "white"

    Loader {
        id: mainLoader
        anchors.fill: parent

        source: Authenticator.isRegistered && Authenticator.isAuthenticated ? "home/Home.qml" : "auth/AuthStack.qml"
        onSourceChanged: console.log("Loader switched to : ", source)
    }

    // FIX: non-existent properties onUidRegistered, onUnlockSuccessful
    // Connections {
    //     target: Authenticator

    //     function onUidRegistered() { console.log("uid registered") }
    //     function onUnlockSuccessful() { console.log("vault unlocked") }
    //     function onError() { console.error(message) }
    // }

    footer: ToolBar {
        RowLayout {
            anchors.fill: parent

            Label {
                text: "UID: " + Authenticator.uid;
                leftPadding: 10
            }

            Item {
                Layout.fillWidth: true
            }
        }
    }
}
