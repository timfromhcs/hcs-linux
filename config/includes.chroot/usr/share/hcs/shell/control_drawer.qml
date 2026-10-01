// HCS Shell - Control Drawer
// Aesthetic: Neural Glass (Obsidian slate, glowing cyan accents, frosted translucency)
// Adheres to GEMINI.md Sections 36-60 (Zero-Electron Mandate)

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Wayland

ShellRoot {
    id: drawerRoot

    PanelWindow {
        id: drawerWindow
        anchors {
            top: true
            bottom: true
            right: true
        }
        margins {
            top: 54
            bottom: 76
            right: 16
        }
        width: 380
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            radius: 20
            color: "#161b22"
            opacity: 0.94
            border.color: "rgba(56, 189, 248, 0.4)"
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 16

                // Header
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "CONTROL DRAWER"
                        font.pixelSize: 13
                        font.bold: true
                        font.letterSpacing: 1.5
                        color: "#38bdf8"
                    }
                    Item { Layout.fillWidth: true }
                    Rectangle {
                        width: 28; height: 28; radius: 14
                        color: "rgba(255, 255, 255, 0.08)"
                        Text {
                            anchors.centerIn: parent
                            text: "x"
                            font.pixelSize: 13
                            color: "#f8fafc"
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: drawerWindow.visible = false
                        }
                    }
                }

                // Section: Tor Isolation & Privacy
                Rectangle {
                    Layout.fillWidth: true
                    height: 64
                    radius: 12
                    color: "rgba(30, 41, 59, 0.6)"
                    border.color: "rgba(192, 132, 252, 0.3)"

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 12

                        Rectangle {
                            width: 38; height: 38; radius: 10
                            color: "#311042"
                            border.color: "#c084fc"
                            Text { anchors.centerIn: parent; text: "TOR"; font.bold: true; font.pixelSize: 10; color: "#c084fc" }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text { text: "Tor Transparent Proxy"; font.bold: true; font.pixelSize: 12; color: "#f8fafc" }
                            Text { text: "Strict zero-leak circuit active"; font.pixelSize: 10; color: "#94a3b8" }
                        }

                        Switch {
                            checked: true
                            onToggled: Quickshell.process("hcs-control").arg("tor").start()
                        }
                    }
                }

                // Section: Cognitive Tier Selector
                Rectangle {
                    Layout.fillWidth: true
                    height: 100
                    radius: 12
                    color: "rgba(30, 41, 59, 0.6)"
                    border.color: "rgba(56, 189, 248, 0.2)"

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "Cognitive Profile Tier"; font.bold: true; font.pixelSize: 12; color: "#f8fafc" }
                            Item { Layout.fillWidth: true }
                            Text { text: "Single-Model Rule Active"; font.pixelSize: 10; color: "#34d399" }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Rectangle {
                                Layout.fillWidth: true; height: 36; radius: 8
                                color: "#0f172a"; border.color: "#38bdf8"; border.width: 1.5
                                Text { anchors.centerIn: parent; text: "Edge (8G)"; color: "#38bdf8"; font.pixelSize: 11; font.bold: true }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: Quickshell.process("hcs").args(["models", "profile", "edge"]).start()
                                }
                            }

                            Rectangle {
                                Layout.fillWidth: true; height: 36; radius: 8
                                color: "#1e293b"; border.color: "rgba(255,255,255,0.1)"; border.width: 1
                                Text { anchors.centerIn: parent; text: "LowRAM (4G)"; color: "#94a3b8"; font.pixelSize: 11 }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: Quickshell.process("hcs").args(["models", "profile", "lowram"]).start()
                                }
                            }

                            Rectangle {
                                Layout.fillWidth: true; height: 36; radius: 8
                                color: "#1e293b"; border.color: "rgba(255,255,255,0.1)"; border.width: 1
                                Text { anchors.centerIn: parent; text: "Max (16G)"; color: "#94a3b8"; font.pixelSize: 11 }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: Quickshell.process("hcs").args(["models", "profile", "workstation"]).start()
                                }
                            }
                        }
                    }
                }

                // Section: Audio & System Volume
                Rectangle {
                    Layout.fillWidth: true
                    height: 70
                    radius: 12
                    color: "rgba(30, 41, 59, 0.6)"
                    border.color: "rgba(255, 255, 255, 0.08)"

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            Text { text: "PipeWire Audio Master"; font.bold: true; font.pixelSize: 12; color: "#f8fafc" }
                            Item { Layout.fillWidth: true }
                            Text { text: "75%"; font.pixelSize: 11; color: "#38bdf8" }
                        }

                        Slider {
                            Layout.fillWidth: true
                            from: 0.0; to: 1.0; value: 0.75
                        }
                    }
                }

                // Section: Quick System Diagnostic & Tools
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        Layout.fillWidth: true; height: 44; radius: 10
                        color: "#0f172a"; border.color: "rgba(56, 189, 248, 0.4)"; border.width: 1
                        RowLayout {
                            anchors.centerIn: parent; spacing: 6
                            Text { text: "Monitor"; color: "#38bdf8"; font.pixelSize: 11; font.bold: true }
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Quickshell.process("hcs-monitor").start()
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true; height: 44; radius: 10
                        color: "#0f172a"; border.color: "rgba(251, 191, 36, 0.4)"; border.width: 1
                        RowLayout {
                            anchors.centerIn: parent; spacing: 6
                            Text { text: "Diagnose"; color: "#fbbf24"; font.pixelSize: 11; font.bold: true }
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Quickshell.process("hcs-diagnose").start()
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true; height: 44; radius: 10
                        color: "#0f172a"; border.color: "rgba(52, 211, 153, 0.4)"; border.width: 1
                        RowLayout {
                            anchors.centerIn: parent; spacing: 6
                            Text { text: "Ingest"; color: "#34d399"; font.pixelSize: 11; font.bold: true }
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Quickshell.process("hcs-rag-ingest").start()
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                // Footer
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "HCS Linux 13.7 Trixie Core"
                    font.pixelSize: 10
                    color: "#475569"
                }
            }
        }
    }
}
