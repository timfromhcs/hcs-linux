// HCS Shell - Quickshell Desktop Environment
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

ShellRoot {
    id: root

    PanelWindow {
        id: topBar
        anchors {
            top: true
            left: true
            right: true
        }
        height: 48
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            anchors.margins: 6
            radius: 12
            color: "#161b22"
            opacity: 0.85
            border.color: "rgba(255, 255, 255, 0.08)"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 16

                // Brand Emblem
                Text {
                    text: "HCS LINUX"
                    font.bold: true
                    font.pixelSize: 14
                    color: "#38bdf8"
                }

                // Workspaces
                Row {
                    spacing: 8
                    Rectangle { width: 24; height: 6; radius: 3; color: "#38bdf8" }
                    Rectangle { width: 12; height: 6; radius: 3; color: "rgba(255,255,255,0.2)" }
                    Rectangle { width: 12; height: 6; radius: 3; color: "rgba(255,255,255,0.2)" }
                }

                Item { Layout.fillWidth: true }

                // Brain Status Indicator
                Rectangle {
                    width: 140
                    height: 28
                    radius: 14
                    color: "rgba(56, 189, 248, 0.15)"
                    border.color: "#38bdf8"
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        Rectangle {
                            width: 8
                            height: 8
                            radius: 4
                            color: "#34d399"
                        }
                        Text {
                            text: "Brain Ready"
                            font.pixelSize: 11
                            color: "#f8fafc"
                        }
                    }
                }

                // Clock
                Text {
                    text: Qt.formatDateTime(new Date(), "hh:mm")
                    font.pixelSize: 13
                    font.bold: true
                    color: "#f8fafc"
                }
            }
        }
    }

    // Glass Floating Dock
    PanelWindow {
        id: dock
        anchors {
            bottom: true
            horizontalCenter: true
        }
        height: 64
        width: 380
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            anchors.bottomMargin: 8
            radius: 18
            color: "rgba(22, 27, 34, 0.85)"
            border.color: "rgba(255, 255, 255, 0.12)"
            border.width: 1

            RowLayout {
                anchors.centerIn: parent
                spacing: 20

                // App Launcher
                Rectangle {
                    width: 40; height: 40; radius: 10
                    color: "#1e293b"
                    Text { anchors.centerIn: parent; text: "Apps"; color: "#38bdf8"; font.pixelSize: 11 }
                }

                // HCS Chat
                Rectangle {
                    width: 40; height: 40; radius: 10
                    color: "#1e293b"
                    Text { anchors.centerIn: parent; text: "Chat"; color: "#818cf8"; font.pixelSize: 11 }
                }

                // HCS Search
                Rectangle {
                    width: 40; height: 40; radius: 10
                    color: "#1e293b"
                    Text { anchors.centerIn: parent; text: "Find"; color: "#34d399"; font.pixelSize: 11 }
                }

                // HCS Settings
                Rectangle {
                    width: 40; height: 40; radius: 10
                    color: "#1e293b"
                    Text { anchors.centerIn: parent; text: "Setup"; color: "#f8fafc"; font.pixelSize: 11 }
                }
            }
        }
    }
}
