import QtQuick 2.15
import QtQuick.Controls
import QtQml.Models
import VaultManager

Item {
    implicitWidth: 400
    implicitHeight: 400
    // parent: SplitView

    // requires layout adjustments
    // VC NOTE: this component is being tested outside of version control and will be refactored afterwards

    Pane {
        anchors.fill: parent

        // structures data from input (user defined groups via Home.qml 'add group' component)
        ListModel {
            id: groupsDataModel
            ListElement {name: "My Vault"}
        }

        DelegateModel {
            id: groupsViewModel
            model: groupsDataModel
            delegate: ItemDelegate {
                width: groupsListView.width
                text: name
            }

            // sorting logic, property name unrelated to groups in the context of the app
            // TODO: sorting (groupsViewModel.item.insert() or DelegateModelGroup filter)
            groups: [
                DelegateModelGroup {name: "visible"}
            ]
        }

        ListView {
            id: groupsListView
            anchors.fill: parent
            topMargin: 20
            clip: false
            model: groupsViewModel
        }

        Button {
            id: addGroupUtility
            anchors.top: parent.top
            anchors.right: parent.right

            // using html to maintain native style
            text: "<font color='black'>Add</font>"

            // for custom styling
            // contentItem: Text {
            //     text: addGroupUtility.text
            //     font: addGroupUtility.font
            //     verticalAlignment: Text.AlignVCenter
            //     horizontalAlignment: Text.AlignHCenter
            //     color: "black"
            // }

            onPressed: addGroupDialog.open()

            Dialog {
                id: addGroupDialog
                title: "Add New Group: "
                standardButtons: Dialog.Ok | Dialog.Cancel
                modal: false

                // allows pane components to call DB_LocalStorage methods and enable persistent data (groups, group removals)
                // NOTE: since VaultManager manages group methods, consider calling addGroup through VaultManager instead
                function addGroup() {
                    let groupName = "";
                    groupName = addGroupDialogInput.text.trim();
                    // TODO: groupName should be limited to around 24 characters
                    if (groupName !== "") {
                        groupsDataModel.append({"name": groupName});

                        // groupID stores the returned id key from SQL while addGroup() executes query
                        let groupID = vaultManager.addGroup(groupName);
                        if (groupID === -1) {
                            console.log("An error occured while attempting to add group to database");
                            return;
                        }

                        addGroupDialogInput.text = "";
                        addGroupDialog.close();
                    }
                }

                Column {
                    spacing: 10
                    width: parent.width / 2

                    TextField {
                        id: addGroupDialogInput
                        width: parent.width
                        placeholderText: "group name"
                        focus: true
                        // on enter pressed
                        onAccepted: addGroupDialog.addGroup()
                    }
                }

                // link to add group to database when Ok
                // onAccepted: VaultDB.addGroup(id, name)

                // on "OK" button pressed
                onAccepted: addGroupDialog.addGroup()
                onRejected: addGroupDialogInput.text = ""

            }
        }


        Text {
            anchors.centerIn: parent
            text: "pane"
        }
    }

    Connections {
        function onInitDBFailure() {
            console.log("Database initialized");
        }

        function onStartDBConnectionFailure(msg) {
            console.log("startDBConnectionFailure: ", msg);
        }

        function onCloseDBConnectionFailure(msg) {
            console.log("closeDBConnectionFailure: ", msg);
        }

        function onStartDBConnectionSuccess() {
            console.log("Database connection successfully opened");
        }

        function onCloseDBConnectionSuccess() {
            console.log("Database connection successfully closed");
        }

        function onDirectoryEstablished(dir) {
            console.log("Database directory established at: ", dir);
        }

        function onDatabasePathEstablished(path) {
            console.log("Database path established at: ", path);
        }

        function onDatabaseQueryFailure(msg) {
            console.log("Query to database failed. error: ", msg);
        }

        function onDatabaseQuerySuccess() {
            console.log("Query to database succeeded");
        }
    }
}

