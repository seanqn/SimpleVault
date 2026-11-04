import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import VaultSecurityServicer

Item {
    id: authContainer

    Column {
        anchors.centerIn: parent

        Button {
            text: "Unlock Vault"
            // APP OUTPUT:
            // QML QQuickRectangle: The current style does not support customization of this control (property: "background")
            background:
                Rectangle {
                    color: "#5b5b5b"
                    radius: 4
                }

            onClicked: {
                if (!Authenticator.isRegistered) {
                    console.log("Navigating to registration...")
                    authLoader.source = "RegisterForm.qml"
                }
                else {
                    console.log("Navigating to authentication...")
                    authLoader.source = "AuthForm.qml"
                }
            }
        }
    }

    Loader {
        id: authLoader
    }

    Connections {
        target: Authenticator

        function onError(message) { return error(message) }
    }
}
