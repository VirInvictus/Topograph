import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import com.topograph 1.0

ApplicationWindow {
    visible: true
    width: 1024
    height: 768
    title: "Topograph"
    color: "#1F1F28" // Kanagawa Sumi Ink 1 (Background)

    ScanBridge {
        id: bridge
    }

    Timer {
        interval: 16 // ~60fps
        running: bridge.isScanning
        repeat: true
        onTriggered: bridge.updateMetrics()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20

        RowLayout {
            Layout.fillWidth: true
            
            TextField {
                id: pathInput
                Layout.fillWidth: true
                text: "/"
                color: "#DCD7BA" // Fuji White
                background: Rectangle {
                    color: "#2A2A37" // Sumi Ink 2
                    radius: 4
                }
            }

            Button {
                text: bridge.isScanning ? "Cancel" : "Scan"
                onClicked: {
                    if (bridge.isScanning) {
                        bridge.cancelScan()
                    } else {
                        bridge.startScan(pathInput.text)
                    }
                }
                background: Rectangle {
                    color: bridge.isScanning ? "#C34043" : "#76946A" // Autumn Red vs Spring Green
                    radius: 4
                }
                contentItem: Text {
                    text: parent.text
                    color: "#DCD7BA"
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: bridge.isScanning || bridge.progressText !== ""
            
            Text {
                text: bridge.progressText
                color: "#957FB8" // Oni Violet
                font.pixelSize: 14
            }

            ProgressBar {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                indeterminate: bridge.isScanning
                visible: bridge.isScanning
                background: Rectangle {
                    color: "#2A2A37"
                    radius: 2
                }
                contentItem: Item {
                    Rectangle {
                        id: progressRect
                        width: parent.width * 0.3
                        height: parent.height
                        color: "#E82424" // Samurai Red
                        radius: 2
                        NumberAnimation on x {
                            from: 0
                            to: progressRect.parent.width * 0.7
                            duration: 1000
                            loops: Animation.Infinite
                            running: bridge.isScanning
                        }
                    }
                }
            }

            Text {
                text: bridge.speedText
                color: "#7E9CD8" // Crystal Blue
                font.pixelSize: 14
            }
        }

        // Content Area
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "#16161D" // Sumi Ink 0
            radius: 8
            clip: true
            
            ListView {
                id: treeView
                anchors.fill: parent
                anchors.margins: 8
                model: dirModel
                
                delegate: Item {
                    width: treeView.width
                    height: 24
                    
                    RowLayout {
                        anchors.fill: parent
                        
                        // Indentation based on depth
                        Item {
                            Layout.preferredWidth: model.depth * 20
                        }
                        
                        Text {
                            text: model.isDirectory ? "📁" : "📄"
                            font.pixelSize: 14
                        }
                        
                        Text {
                            text: model.fileName
                            color: "#DCD7BA"
                            font.pixelSize: 14
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }
                        
                        Text {
                            text: {
                                let mb = model.fileSize / (1024 * 1024);
                                return mb.toFixed(2) + " MB";
                            }
                            color: "#957FB8"
                            font.pixelSize: 14
                            horizontalAlignment: Text.AlignRight
                        }
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                text: bridge.isScanning ? "Scanning..." : "Ready."
                color: "#54546D"
                font.pixelSize: 24
                visible: treeView.count === 0
            }
        }
    }

    DirectoryModel {
        id: dirModel
    }

    Connections {
        target: bridge
        function onIsScanningChanged() {
            if (!bridge.isScanning && bridge.progressText === "Scan complete.") {
                dirModel.loadTree()
            }
        }
    }
}
