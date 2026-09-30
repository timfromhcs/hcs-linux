// HCS Linux — Stage Manager (v2 plan §5 W2.6)
// Pattern: macOS Stage Manager. One app stays front and centre; the rest are
// reduced to a left-hand strip so the desktop never becomes wallpaper soup.
// Unlike macOS this is reversible at any time and groups windows like App Intents.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

FloatingWindow {
    id: stage
    anchors { top: true; bottom: true; left: true }
    width: 168
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    readonly property bool active: staged

    // Windows that are not in the foreground live here as icon strips, newest
    // first. Grouping mirrors macOS: dragging an app onto the stage group opens
    // the whole set together.
    readonly property var strip: [
        { app: "hcs-monitor", title: "System Monitor" },
        { app: "hcs-fm",     title: "Files" },
        { app: "hcs-term",   title: "Terminal" },
        { app: "hcs-docs",   title: "Documentation" },
        { app: "hcs-search", title: "Omnibar" },
        { app: "hcs-chat",   title: "AI Chat" }
    ]

    Rectangle {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.topMargin: 64
        anchors.bottomMargin: 64
        anchors.rightMargin: 8
        radius: 18
        color: "#0d1117"
        opacity: 0.88
        border.color: "rgba(255,255,255,0.08)"
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 8

            Text {
                text: "Stage"
                color: "#8b949e"
                font.family: "Inter"
                font.pixelSize: 10
                font.bold: true
                Layout.alignment: Qt.AlignHCenter
            }

            Repeater {
                model: stage.strip
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.preferredHeight: 52
                    radius: 10
                    color: "#161b22"
                    border.color: "rgba(56,189,248,0.22)"
                    border.width: 1

                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 3
                        Rectangle {
                            Layout.alignment: Qt.AlignHCenter
                            width: 18; height: 18; radius: 5
                            color: "#38bdf8"
                        }
                        Text {
                            text: modelData.title
                            color: "#e6edf3"
                            font.family: "Inter"
                            font.pixelSize: 9
                            elide: Text.ElideRight
                            Layout.maximumWidth: 128
                        }
                    }

                    TapHandler {
                        onTapped: {
                            Quickshell.execDetached(
                                ["/usr/bin/hcs", "window", "stage", "--app", modelData.app])
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }

            // Toggling stage off restores normal free-floating windows.
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: "HCS+ALT+S"
                color: "#4b5563"
                font.family: "Inter"
                font.pixelSize: 9
            }
        }
    }
}
