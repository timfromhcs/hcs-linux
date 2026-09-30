// HCS Shell - Semantic Clipboard Manager
// Aesthetic: Neural Glass (Obsidian slate, glowing cyan accents, redacted secret safety)
// Adheres to GEMINI.md Sections 36-60 (Zero-Electron Mandate)

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Wayland

ShellRoot {
    id: clipRoot

    PanelWindow {
        id: clipWindow
        anchors {
            horizontalCenter: true
            verticalCenter: true
        }
        width: 520
        height: 380
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            radius: 18
            color: "#161b22"
            opacity: 0.96
            border.color: "rgba(56, 189, 248, 0.4)"
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 12

                // Header
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "SEMANTIC CLIPBOARD"
                        font.pixelSize: 12
                        font.bold: true
                        font.letterSpacing: 1.5
                        color: "#38bdf8"
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: "Esc to close"
                        font.pixelSize: 11
                        color: "#64748b"
                    }
                }

                // Search Bar
                Rectangle {
                    Layout.fillWidth: true
                    height: 38
                    radius: 8
                    color: "#0f172a"
                    border.color: "rgba(255, 255, 255, 0.12)"
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 8

                        Text {
                            text: "🔍"
                            font.pixelSize: 12
                            color: "#94a3b8"
                        }

                        TextInput {
                            id: searchInput
                            Layout.fillWidth: true
                            color: "#f8fafc"
                            font.pixelSize: 12
                            focus: true
                            clip: true
                            // Placeholder
                            Text {
                                text: "Search clipboard history..."
                                color: "#475569"
                                font.pixelSize: 12
                                visible: !searchInput.text && !searchInput.activeFocus
                            }
                        }
                    }
                }

                // Clipboard Items List
                ListView {
                    id: clipList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 6

                    model: ListModel {
                        id: clipModel
                        ListElement {
                            clipText: "cargo test --workspace"
                            time: "1m ago"
                            isRedacted: false
                            category: "command"
                        }
                        ListElement {
                            clipText: "[REDACTED TOKEN: ghp_****...]"
                            time: "5m ago"
                            isRedacted: true
                            category: "secret"
                        }
                        ListElement {
                            clipText: "https://huggingface.co/Qwen/Qwen3-0.6B"
                            time: "12m ago"
                            isRedacted: false
                            category: "url"
                        }
                        ListElement {
                            clipText: "Debian 13.7 (codename trixie) live-build ISO"
                            time: "25m ago"
                            isRedacted: false
                            category: "text"
                        }
                    }

                    delegate: Rectangle {
                        width: clipList.width
                        height: 48
                        radius: 8
                        color: isRedacted ? "rgba(225, 29, 72, 0.1)" : (mouseArea.containsMouse ? "rgba(56, 189, 248, 0.15)" : "#1e293b")
                        border.color: isRedacted ? "#f43f5e" : (mouseArea.containsMouse ? "#38bdf8" : "rgba(255, 255, 255, 0.06)")
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                width: 28; height: 28; radius: 6
                                color: isRedacted ? "#881337" : "#0f172a"
                                Text {
                                    anchors.centerIn: parent
                                    text: isRedacted ? "🔒" : (category === "command" ? ">_" : "📋")
                                    font.pixelSize: 11
                                    color: isRedacted ? "#fb7185" : "#38bdf8"
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: clipText
                                font.pixelSize: 11
                                font.family: "JetBrains Mono, monospace"
                                color: isRedacted ? "#fb7185" : "#f8fafc"
                                elide: Text.ElideRight
                            }

                            Text {
                                text: time
                                font.pixelSize: 10
                                color: "#64748b"
                            }
                        }

                        MouseArea {
                            id: mouseArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                // Paste / copy to clipboard action
                                clipWindow.visible = false
                            }
                        }
                    }
                }
            }
        }
    }
}
