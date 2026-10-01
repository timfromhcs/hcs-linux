// HCS Linux — Control Center (v2 plan §5 W4.2)
// Pattern: macOS Control Center gallery. Every module is a control that can be
// added or removed; the chosen set persists per user.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: controlCenter
    anchors { top: true; right: true }
    width: 360
    height: 460
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

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

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Text {
                    text: "Control Center"
                    color: "#e6edf3"
                    font.family: "Inter"
                    font.pixelSize: 15
                    font.bold: true
                    Layout.fillWidth: true
                }
                Text {
                    text: "HCS+E"
                    color: "#8b949e"
                    font.family: "Inter"
                    font.pixelSize: 11
                }
            }

            // --- connectivity + privacy cluster -------------------------------
            GridLayout {
                Layout.fillWidth: true
                columns: 4
                columnSpacing: 8
                rowSpacing: 8

                Repeater {
                    model: [
                        { label: "Tor";      glyph: "🧅"; state: "ISOLATED"; ok: false },
                        { label: "Network";  glyph: "🌐"; state: "Wired";    ok: true  },
                        { label: "Audio";    glyph: "🔊"; state: "48%";      ok: true  },
                        { label: "Privacy";  glyph: "🔒"; state: "Strict";   ok: true  }
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        Layout.preferredHeight: 74
                        radius: 14
                        color: "#161b22"
                        border.color: modelData.ok ? "rgba(56,189,248,0.35)" : "rgba(248,81,73,0.45)"
                        border.width: 1

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 4
                            Text {
                                text: modelData.glyph
                                font.pixelSize: 20
                            }
                            Text {
                                text: modelData.label
                                color: "#8b949e"
                                font.family: "Inter"
                                font.pixelSize: 10
                            }
                            Text {
                                text: modelData.state
                                color: modelData.ok ? "#3fb950" : "#f59e0b"
                                font.family: "Inter"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }
                    }
                }
            }

            // --- system sliders ------------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10

                Repeater {
                    model: [
                        { label: "Brightness"; value: 0.72; accent: "#38bdf8" },
                        { label: "Volume";     value: 0.48; accent: "#34d399" },
                        { label: "AI Footprint"; value: 0.31; accent: "#a78bfa" }
                    ]
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 4

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: modelData.label
                                color: "#8b949e"
                                font.family: "Inter"
                                font.pixelSize: 11
                                Layout.fillWidth: true
                            }
                            Text {
                                text: Math.round(modelData.value * 100) + "%"
                                color: "#e6edf3"
                                font.family: "Inter"
                                font.pixelSize: 11
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            radius: 3
                            color: "#1c2430"
                            Rectangle {
                                width: parent.width * modelData.value
                                height: parent.height
                                radius: 3
                                color: modelData.accent
                            }
                        }
                    }
                }
            }

            // --- profile + budget ----------------------------------------------
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 92
                radius: 14
                color: "#161b22"
                border.color: "rgba(255,255,255,0.07)"
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 6

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "AI Profile"
                            color: "#8b949e"
                            font.family: "Inter"
                            font.pixelSize: 11
                            Layout.fillWidth: true
                        }
                        Text {
                            text: "EDGE-8GB"
                            color: "#38bdf8"
                            font.family: "Inter"
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }

                    Text {
                        text: "RAM budget"
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
                            width: parent.width * 0.34
                            height: parent.height
                            radius: 3
                            color: "#34d399"
                        }
                    }
                    Text {
                        text: "2100 MB / 6144 MB idle"
                        color: "#8b949e"
                        font.family: "Inter"
                        font.pixelSize: 10
                    }
                }
            }

            Item { Layout.fillHeight: true }
        }
    }
}
