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
        running: bridge.is_scanning
        repeat: true
        onTriggered: bridge.update_metrics()
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
                text: bridge.is_scanning ? "Cancel" : "Scan"
                onClicked: {
                    if (bridge.is_scanning) {
                        bridge.cancel_scan()
                    } else {
                        bridge.start_scan(pathInput.text)
                    }
                }
                background: Rectangle {
                    color: bridge.is_scanning ? "#C34043" : "#76946A" // Autumn Red vs Spring Green
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
            visible: bridge.is_scanning || bridge.progress_text !== ""
            
            Text {
                text: bridge.progress_text
                color: "#957FB8" // Oni Violet
                font.pixelSize: 14
            }

            ProgressBar {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                indeterminate: bridge.is_scanning
                visible: bridge.is_scanning
                background: Rectangle {
                    color: "#2A2A37"
                    radius: 2
                }
                contentItem: Item {
                    Rectangle {
                        width: parent.width * 0.3
                        height: parent.height
                        color: "#E82424" // Samurai Red
                        radius: 2
                        NumberAnimation on x {
                            from: 0
                            to: parent.width * 0.7
                            duration: 1000
                            loops: Animation.Infinite
                            running: bridge.is_scanning
                        }
                    }
                }
            }

            Text {
                text: bridge.speed_text
                color: "#7E9CD8" // Crystal Blue
                font.pixelSize: 14
            }
        }

        // Placeholder for the visualization & tree view
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "#16161D" // Sumi Ink 0 (Darker background for content)
            radius: 8
            
            Text {
                anchors.centerIn: parent
                text: bridge.is_scanning ? "Scanning..." : "Ready."
                color: "#54546D" // Sumi Ink 4 (Disabled text)
                font.pixelSize: 24
            }
        }
    }
}
