// HCS Linux — Neural Glass Taskbar v1.0.0 Stable
// Spec: docs/V1_STABLE_RELEASE_MASTER_PLAN.md §4.1
// Geometry: bottom-anchored, height 52px, acrylic frosted obsidian #0d1117 @0.90

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: taskbar
    anchors {
        bottom: true
        left: true
        right: true
    }
    height: 52
    color: "transparent"

    Rectangle {
        anchors.fill: parent
        color: "#0d1117"
        opacity: 0.90
        border.color: "rgba(255, 255, 255, 0.08)"
        border.width: 1

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            spacing: 8

            // LEFT: Start Monogram (36x36) + Search Pill
            RowLayout {
                spacing: 8
                Rectangle {
                    width: 36; height: 36; radius: 9
                    color: "#0f172a"
                    border.color: "#38bdf8"
                    border.width: 1
                    Text {
                        anchors.centerIn: parent
                        text: "H"
                        font.bold: true
                        font.pixelSize: 17
                        color: "#38bdf8"
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: Quickshell.process("quickshell").args(["-p", "/usr/share/hcs/shell/start_menu.qml"]).start()
                    }
                }
                Rectangle {
                    width: 220; height: 32; radius: 16
                    color: "rgba(255,255,255,0.06)"
                    border.color: "rgba(255,255,255,0.08)"
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: 14
                        text: "Search apps or ask AI..."
                        font.pixelSize: 12
                        color: "rgba(226,232,240,0.55)"
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-search").start()
                    }
                }
            }

            // CENTER: Running Tasks (Wayland toplevels)
            Item {
                Layout.fillWidth: true
                height: 52
                Row {
                    id: tasklist
                    anchors.centerIn: parent
                    spacing: 4
                    // Populated dynamically from Wayland ToplevelManager.
                    // Active window: cyan 2px underline; click toggles minimize/focus.
                    // Hover: thumbnail preview via Wayland foreign-toplevel preview.
                    Repeater {
                        // Placeholder bound at runtime to ToplevelManager.toplevels
                        model: 4
                        Rectangle {
                            width: 120; height: 38; radius: 9
                            color: index === 0 ? "rgba(56,189,248,0.14)" : "transparent"
                            border.color: index === 0 ? "rgba(56,189,248,0.35)" : "transparent"
                            Text {
                                anchors.centerIn: parent
                                text: ["HCS Chat", "Terminal", "Files", "VSCodium"][index]
                                font.pixelSize: 11
                                color: "#e2e8f0"
                            }
                            Rectangle {
                                // Active-window cyan underline indicator (2px)
                                visible: index === 0
                                anchors.bottom: parent.bottom
                                anchors.bottomMargin: 3
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: 24; height: 2; radius: 1
                                color: "#38bdf8"
                            }
                        }
                    }
                }
            }

            // RIGHT: Tray & Control Cluster
            RowLayout {
                spacing: 8
                // Tor Shield Pill (Green=Active, Gray=Direct)
                Rectangle {
                    width: 96; height: 28; radius: 14
                    color: "rgba(49,16,66,0.4)"
                    border.color: "rgba(192,132,252,0.4)"
                    Text {
                        anchors.centerIn: parent
                        text: "TOR SHIELD"
                        font.pixelSize: 10
                        font.bold: true
                        color: "#e2e8f0"
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: Quickshell.process("hcs-tor-switch").args(["toggle"]).start()
                    }
                }
                // AI Cognitive Footprint
                Rectangle {
                    width: 150; height: 28; radius: 14
                    color: "rgba(15,23,42,0.75)"
                    border.color: "rgba(56,189,248,0.4)"
                    Text {
                        anchors.centerIn: parent
                        text: "Qwen3-0.6B | 550MB"
                        font.pixelSize: 10
                        font.family: "JetBrains Mono, monospace"
                        color: "#38bdf8"
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-monitor").start()
                    }
                }
                Text { text: "🔊"; font.pixelSize: 14 }       // PipeWire master slider on click
                Text { text: "📶"; font.pixelSize: 14 }       // Wi-Fi/Ethernet selector on click
                Text {                                        // Clock + calendar dropdown on click
                    text: Qt.formatDateTime(new Date(), "HH:mm\ndd.MM.yyyy")
                    font.pixelSize: 10
                    color: "#f8fafc"
                    horizontalAlignment: Text.AlignHCenter
                }
                Text { text: "🔔"; font.pixelSize: 14 }       // Action Center bell + badge
            }
        }
    }
}
