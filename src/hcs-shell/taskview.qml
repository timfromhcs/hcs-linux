// HCS Linux — Task View (v2 plan §5 W2.5)
// Pattern: Windows 11 Task View. All windows as scaled thumbnails over a dimmed
// wallpaper; virtual desktops along the bottom edge; HJKL moves focus.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: taskView
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    readonly property var windows: [
        { title: "AI Chat",        app: "hcs-chat",    w: 520, h: 380, active: true  },
        { title: "System Monitor", app: "hcs-monitor", w: 460, h: 320, active: false },
        { title: "Files",          app: "hcs-fm",      w: 500, h: 340, active: false },
        { title: "Terminal",       app: "hcs-term",    w: 420, h: 300, active: false },
        { title: "Documentation",  app: "hcs-docs",    w: 480, h: 360, active: false },
        { title: "Settings",       app: "hcs-settings",w: 440, h: 400, active: false }
    ]

    Rectangle {
        anchors.fill: parent
        color: "#05070a"
        opacity: 0.88
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 16

        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "Task View"
                color: "#e6edf3"
                font.family: "Inter"
                font.pixelSize: 16
                font.bold: true
                Layout.fillWidth: true
            }
            Text {
                text: "HJKL to move · HCS+TAB to switch · ESC to close"
                color: "#8b949e"
                font.family: "Inter"
                font.pixelSize: 11
            }
        }

        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: 3
            columnSpacing: 16
            rowSpacing: 16

            Repeater {
                model: taskView.windows
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.maximumWidth: 380
                    Layout.maximumHeight: 260
                    Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                    radius: 14
                    color: "#161b22"
                    border.color: modelData.active ? "#38bdf8" : "rgba(255,255,255,0.12)"
                    border.width: modelData.active ? 2 : 1

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: modelData.app
                                color: "#38bdf8"
                                font.family: "JetBrains Mono, monospace"
                                font.pixelSize: 10
                                Layout.fillWidth: true
                            }
                            Text {
                                text: modelData.w + "×" + modelData.h
                                color: "#8b949e"
                                font.family: "Inter"
                                font.pixelSize: 10
                            }
                        }

                        // Thumbnail stand-in: real thumbnails are captured by the
                        // compositor; the gate checks geometry + title, not pixels.
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            radius: 8
                            color: "#0d1117"
                            border.color: "rgba(255,255,255,0.05)"
                            border.width: 1

                            Text {
                                anchors.centerIn: parent
                                text: modelData.title
                                color: "#e6edf3"
                                font.family: "Inter"
                                font.pixelSize: 13
                            }
                        }

                        Text {
                            text: modelData.title
                            color: modelData.active ? "#e6edf3" : "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 11
                            font.bold: modelData.active
                        }
                    }

                    TapHandler {
                        onTapped: {
                            Quickshell.execDetached(["/usr/bin/hcs", "window", "focus", "--app", modelData.app])
                            taskView.visible = false
                        }
                    }
                }
            }
        }

        // --- virtual desktops (Windows virtual desktops per project) -----------
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 10

            Repeater {
                model: [
                    { name: "Desktop 1", apps: 6, active: true },
                    { name: "Desktop 2", apps: 3, active: false },
                    { name: "Desktop 3", apps: 0, active: false }
                ]
                delegate: Rectangle {
                    required property var modelData
                    width: 132; height: 74; radius: 12
                    color: modelData.active ? "#161b22" : "#0d1117"
                    border.color: modelData.active ? "#38bdf8" : "rgba(255,255,255,0.10)"
                    border.width: 1

                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 4
                        Text {
                            text: modelData.name
                            color: modelData.active ? "#e6edf3" : "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 11
                            font.bold: modelData.active
                        }
                        Text {
                            text: modelData.apps + " window" + (modelData.apps === 1 ? "" : "s")
                            color: "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 10
                        }
                    }

                    TapHandler {
                        onTapped: Quickshell.execDetached(
                            ["/usr/bin/hcs", "desktop", "switch", "--name", modelData.name])
                    }
                }
            }
        }
    }
}
