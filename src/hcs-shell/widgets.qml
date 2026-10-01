// HCS Linux — Widgets board (v2 plan §5 W4.4)
// Patterns:
//   - Windows 11: widgets board with a navigation pane, suggestions for empty
//     slots, and lock-screen widgets
//   - iOS 26: widget stacks that adapt to context
//   - GNOME 49: lock-screen media controls without unlocking
//
// The board is glanceable information. It must never be the only place a piece
// of information exists, so every widget opens the full app on click.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

FloatingWindow {
    id: widgetsBoard
    anchors { top: true; left: true }
    width: 420
    height: 480
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    Rectangle {
        anchors.fill: parent
        radius: 20
        color: "#0d1117"
        opacity: 0.94
        border.color: "rgba(255,255,255,0.10)"
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text {
                    text: "Widgets"
                    color: "#e6edf3"
                    font.family: "Inter"
                    font.pixelSize: 15
                    font.bold: true
                    Layout.fillWidth: true
                }
                Text {
                    text: "HCS+W"
                    color: "#8b949e"
                    font.family: "Inter"
                    font.pixelSize: 10
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 12
                rowSpacing: 12

                // --- weather -----------------------------------------------------
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 118
                    radius: 16
                    color: "#161b22"
                    border.color: "rgba(255,255,255,0.07)"
                    border.width: 1
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 6
                        Text {
                            text: "Berlin"
                            color: "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 11
                        }
                        Text {
                            text: "18°"
                            color: "#e6edf3"
                            font.family: "Inter"
                            font.pixelSize: 30
                            font.bold: true
                        }
                        Text {
                            text: "Clear · offline forecast cache"
                            color: "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 10
                        }
                    }
                }

                // --- system ------------------------------------------------------
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 118
                    radius: 16
                    color: "#161b22"
                    border.color: "rgba(255,255,255,0.07)"
                    border.width: 1
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 8
                        Text {
                            text: "System"
                            color: "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 11
                        }
                        Repeater {
                            model: [
                                { k: "RAM", v: "2100 MB", c: "#34d399" },
                                { k: "Model", v: "Qwen3-0.6B", c: "#38bdf8" },
                                { k: "Tor", v: "ISOLATED", c: "#f59e0b" }
                            ]
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                Text {
                                    text: modelData.k
                                    color: "#8b949e"
                                    font.family: "Inter"
                                    font.pixelSize: 11
                                    Layout.fillWidth: true
                                }
                                Text {
                                    text: modelData.v
                                    color: modelData.c
                                    font.family: "Inter"
                                    font.pixelSize: 11
                                    font.bold: true
                                }
                            }
                        }
                    }
                }

                // --- calendar ---------------------------------------------------
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 130
                    radius: 16
                    color: "#161b22"
                    border.color: "rgba(255,255,255,0.07)"
                    border.width: 1
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 6
                        Text {
                            text: "Wednesday 30"
                            color: "#e6edf3"
                            font.family: "Inter"
                            font.pixelSize: 13
                            font.bold: true
                        }
                        Repeater {
                            model: [
                                { t: "09:00", e: "Team sync" },
                                { t: "11:30", e: "RAG index rebuild" },
                                { t: "15:00", e: "Release gate review" }
                            ]
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: 10
                                Text {
                                    text: modelData.t
                                    color: "#38bdf8"
                                    font.family: "JetBrains Mono, monospace"
                                    font.pixelSize: 10
                                }
                                Text {
                                    text: modelData.e
                                    color: "#8b949e"
                                    font.family: "Inter"
                                    font.pixelSize: 10
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }

                // --- privacy ----------------------------------------------------
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 130
                    radius: 16
                    color: "#161b22"
                    border.color: "rgba(63,185,80,0.28)"
                    border.width: 1
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 8
                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: "🧅"
                                font.pixelSize: 18
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: "PROTECTED"
                                color: "#3fb950"
                                font.family: "Inter"
                                font.pixelSize: 10
                                font.bold: true
                            }
                        }
                        Text {
                            text: "Kill switch active · 0 DNS leaks"
                            color: "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 10
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                        }
                        Text {
                            text: "Amnesic session off"
                            color: "#4b5563"
                            font.family: "Inter"
                            font.pixelSize: 10
                            Layout.fillWidth: true
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "Widgets are suggestions, never the only place data lives — click to open the full app."
                color: "#4b5563"
                font.family: "Inter"
                font.pixelSize: 9
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                Layout.maximumWidth: 320
            }
        }
    }
}
