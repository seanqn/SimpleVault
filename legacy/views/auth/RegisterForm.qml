import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import VaultSecurityServicer

// TODO: requires account removal components and to test registration

Item {
    function registrationMethod() {
        if (passwordInputField.text && passwordInputField.text.length <= 24) {
            console.log("Password accepted");
            Authenticator.authenticateUser(passwordInputField.text.trim())
        }
        else {
            // error
            console.log("Invalid password");
        }
    }

    Column {
        spacing: 20

        Rectangle {
            id: registrationBox
            anchors.centerIn: parent
            width: 200
            height: 300

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
                    registrationMethod()
                }
            }
        }
    }
}
