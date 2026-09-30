// HCS Linux — Notification Center (v2 plan §5 W4.3)
// Patterns combined:
//   - Windows 11: Focus Sessions / DND live in Quick Settings, not a buried menu
//   - GNOME 49  : Do Not Disturb moved into the quick-settings panel; per-monitor
//                 brightness; grouped notifications
//   - Android 16: Live Updates as progress-centric persistent chips, so the user
//                 never has to open the app to learn the status
//   - iOS 26    : Focus + screening of unknown senders
//
// Live Activities are progress notifications (image render, Tor handshake, update).
// They persist until resolved and are the only notifications allowed to pin.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: notificationCenter
    anchors { top: true; right: true }
    width: 380
    height: 520
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    readonly property bool dnd: dndSwitch.checked

    Rectangle {
        anchors.fill: parent
        radius: 18
        color: "#0d1117"
        opacity: 0.94
        border.color: "rgba(255,255,255,0.10)"
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 12

            // --- header: focus mode -------------------------------------------
            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    text: "Notifications"
                    color: "#e6edf3"
                    font.family: "Inter"
                    font.pixelSize: 15
                    font.bold: true
                    Layout.fillWidth: true
                }

                // Do Not Disturb lives here, per GNOME 49 + Win11 Focus Sessions.
                Rectangle {
                    width: 96; height: 26; radius: 13
                    color: notificationCenter.dnd ? "#f59e0b" : "#1c2430"
                    border.color: "rgba(255,255,255,0.12)"
                    border.width: 1
                    Text {
                        anchors.centerIn: parent
                        text: notificationCenter.dnd ? "DND ON" : "Focus"
                        color: notificationCenter.dnd ? "#0d1117" : "#8b949e"
                        font.family: "Inter"
                        font.pixelSize: 10
                        font.bold: true
                    }
                    TapHandler { onTapped: dndSwitch.checked = !dndSwitch.checked }
                }
            }

            Switch {
                id: dndSwitch
                visible: false
                checked: false
            }

            // --- live activities (Android Live Updates pattern) ----------------
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 92
                radius: 14
                color: "#161b22"
                border.color: "rgba(56,189,248,0.40)"
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Rectangle {
                            width: 8; height: 8; radius: 4; color: "#38bdf8"
                        }
                        Text {
                            text: "Image Studio"
                            color: "#e6edf3"
                            font.family: "Inter"
                            font.pixelSize: 12
                            font.bold: true
                            Layout.fillWidth: true
                        }
                        Text {
                            text: "LIVE"
                            color: "#38bdf8"
                            font.family: "Inter"
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }

                    Text {
                        text: "Rendering 512×512 · step 6/8"
                        color: "#8b949e"
                        font.family: "Inter"
                        font.pixelSize: 11
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 6
                        radius: 3
                        color: "#1c2430"
                        Rectangle {
                            width: parent.width * 0.75
                            height: parent.height
                            radius: 3
                            color: "#38bdf8"
                        }
                    }
                }
            }

            // --- grouped standard notifications (Android auto-grouping) -------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: "Earlier · grouped"
                    color: "#8b949e"
                    font.family: "Inter"
                    font.pixelSize: 10
                }

                Repeater {
                    model: 3
                    delegate: Rectangle {
                        required property int index
                        Layout.fillWidth: true
                        Layout.preferredHeight: 46
                        radius: 12
                        color: "#161b22"
                        border.color: "rgba(255,255,255,0.06)"
                        border.width: 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            spacing: 10

                            Rectangle {
                                width: 22; height: 22; radius: 6
                                color: index === 0 ? "#38bdf8" : (index === 1 ? "#34d399" : "#a78bfa")
                            }
                            ColumnLayout {
                                spacing: 1
                                Text {
                                    text: ["hcsd", "Tor switch", "Update"][index]
                                    color: "#e6edf3"
                                    font.family: "Inter"
                                    font.pixelSize: 11
                                }
                                Text {
                                    text: ["Socket ready", "Killswitch armed · 0 DNS leaks",
                                           "Level 1 ready · reboot required"][index]
                                    color: "#8b949e"
                                    font.family: "Inter"
                                    font.pixelSize: 10
                                }
                            }
                            Item { Layout.fillWidth: true }
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "Clear all"
                color: "#8b949e"
                font.family: "Inter"
                font.pixelSize: 11
            }
        }
    }
}
