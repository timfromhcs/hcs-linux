// HCS Shell - Notifications Center
// Aesthetic: Neural Glass (Obsidian slate, glowing cyan/rose accents, acrylic blur)
// Adheres to GEMINI.md Sections 36-60 (Zero-Electron Mandate)

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Wayland

ShellRoot {
    id: notificationRoot

    PanelWindow {
        id: notifyWindow
        anchors {
            top: true
            right: true
        }
        margins {
            top: 56
            right: 16
        }
        width: 360
        height: notificationList.count > 0 ? Math.min(notificationList.count * 88 + 50, 480) : 0
        color: "transparent"
        visible: notificationList.count > 0

        Rectangle {
            anchors.fill: parent
            radius: 16
            color: "#161b22"
            opacity: 0.92
            border.color: "rgba(56, 189, 248, 0.3)"
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                // Header
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "NOTIFICATIONS"
                        font.pixelSize: 11
                        font.bold: true
                        font.letterSpacing: 1.2
                        color: "#94a3b8"
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: "Clear All"
                        font.pixelSize: 11
                        color: "#38bdf8"
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: notificationModel.clear()
                        }
                    }
                }

                // Notifications ListView
                ListView {
                    id: notificationList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 8
                    model: ListModel {
                        id: notificationModel
                        ListElement {
                            nid: 1
                            title: "HCS Brain"
                            body: "Autonomous cognitive cycle 42 completed successfully."
                            urgency: "normal"
                            time: "Just now"
                        }
                        ListElement {
                            nid: 2
                            title: "Security Shield"
                            body: "Tor Transparent Circuit established (3 hops active)."
                            urgency: "low"
                            time: "2m ago"
                        }
                    }

                    delegate: Rectangle {
                        width: notificationList.width
                        height: 72
                        radius: 10
                        color: urgency === "critical" ? "rgba(225, 29, 72, 0.15)" : "rgba(30, 41, 59, 0.7)"
                        border.color: urgency === "critical" ? "#f43f5e" : "rgba(255, 255, 255, 0.08)"
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 36
                                height: 36
                                radius: 8
                                color: urgency === "critical" ? "#e11d48" : "#0284c7"
                                Text {
                                    anchors.centerIn: parent
                                    text: urgency === "critical" ? "!" : "i"
                                    font.bold: true
                                    font.pixelSize: 14
                                    color: "#ffffff"
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                RowLayout {
                                    Layout.fillWidth: true
                                    Text {
                                        text: model.title
                                        font.bold: true
                                        font.pixelSize: 12
                                        color: "#f8fafc"
                                    }
                                    Item { Layout.fillWidth: true }
                                    Text {
                                        text: model.time
                                        font.pixelSize: 10
                                        color: "#64748b"
                                    }
                                }

                                Text {
                                    text: model.body
                                    font.pixelSize: 11
                                    color: "#cbd5e1"
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }

                            // Dismiss button
                            Text {
                                text: "x"
                                font.pixelSize: 12
                                color: "#94a3b8"
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: notificationModel.remove(index)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
