import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import VaultSecurityServicer

Item {
    id: authForm
    anchors.fill: parent

    Popup {
        id: authOverlay

        anchors.centerIn: Overlay.overlay
        width: 250
        height: 300
        visible: true
        modal: true
        focus: true
        closePolicy: Popup.NoAutoClose

        TextField {
            id: passwordInputField

            anchors.centerIn: parent
            placeholderText: "Password"
            placeholderTextColor: "#D3D3D3"
            color: "black"
            echoMode: TextInput.Password
            passwordMaskDelay: 1000
            maximumLength: 24

            onAccepted: {
                Authenticator.authenticateUser(passwordInputField.text.trim())
            }
        }
    }

    // handle logic upon valid authentication
    Connections {
        target: Authenticator

        function onAuthSuccessful() {
            console.log("Authentication successful. Navigating to vault...");
            authOverlay.close();
        }

        function onError(message) { return error(message); }
    }
}
