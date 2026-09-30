// HCS Linux — Silicon Valley Start Menu v1.0.0 Stable
// Spec: docs/V1_STABLE_RELEASE_MASTER_PLAN.md §4.1
// Geometry: bottom-left above taskbar, 560x640, radius 18px

import QtQuick
import QtQuick.Layouts
import Quickshell

PanelWindow {
    id: startMenu
    anchors {
        bottom: true
        left: true
    }
    margins.bottom: 60
    margins.left: 8
    width: 560
    height: 640
    color: "transparent"

    Rectangle {
        anchors.fill: parent
        radius: 18
        color: "#0d1117"
        opacity: 0.96
        border.color: "rgba(255,255,255,0.10)"
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12

            // TOP HEADER: Omnisearch (apps, files, "Ask Brain...")
            Rectangle {
                Layout.fillWidth: true
                height: 44; radius: 12
                color: "rgba(255,255,255,0.06)"
                border.color: "rgba(56,189,248,0.35)"
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: 14
                    text: "🔍  Search apps, files, or Ask Brain..."
                    font.pixelSize: 13
                    color: "rgba(226,232,240,0.6)"
                }
            }

            // MIDDLE: Pinned apps (4-col grid) + Recent files / memories
            Grid {
                Layout.fillWidth: true
                columns: 4
                rowSpacing: 10
                columnSpacing: 10
                Repeater {
                    model: ["HCS Chat", "Image Studio", "Security Lab", "Files", "Settings", "Terminal", "VSCodium", "Tor Browser"]
                    Rectangle {
                        width: 124; height: 72; radius: 12
                        color: "#161b22"
                        border.color: "rgba(255,255,255,0.08)"
                        Column {
                            anchors.centerIn: parent
                            spacing: 4
                            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "▣"; font.pixelSize: 20; color: "#38bdf8" }
                            Text { anchors.horizontalCenter: parent.horizontalCenter; text: modelData; font.pixelSize: 11; color: "#e2e8f0" }
                        }
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                height: 140; radius: 12
                color: "rgba(255,255,255,0.03)"
                Text {
                    anchors.fill: parent
                    anchors.margins: 12
                    text: "Recent files & knowledge nodes\n• Q3 pentest report.md\n• tor-opsec checklist\n• render-042.png (Image Studio)"
                    font.pixelSize: 11
                    color: "rgba(226,232,240,0.75)"
                }
            }

            Item { Layout.fillHeight: true }

            // BOTTOM FOOTER: avatar + power actions
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Rectangle { width: 32; height: 32; radius: 16; color: "#1e293b"; Text { anchors.centerIn: parent; text: "👤"; font.pixelSize: 16 } }
                Text { text: "hcs-user"; font.pixelSize: 12; color: "#f8fafc"; Layout.fillWidth: true }
                Row {
                    spacing: 8
                    Repeater {
                        model: ["Lock", "Sleep", "Restart", "Power Off"]
                        Rectangle {
                            width: 76; height: 30; radius: 15
                            color: "rgba(255,255,255,0.06)"
                            Text { anchors.centerIn: parent; text: modelData; font.pixelSize: 11; color: "#e2e8f0" }
                        }
                    }
                }
            }
        }
    }
}
