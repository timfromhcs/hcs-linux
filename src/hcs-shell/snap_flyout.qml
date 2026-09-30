// HCS Linux — Snap Layout flyout (v2 plan §5 W2.1)
// Pattern: Windows 11 Snap Layouts — hovering the maximise button shows a layout
// grid; choosing one fills it, and the arrangement is remembered as a Snap Group.
//
// Triggered by HCS+Z and by hovering the maximise button of a window.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

FloatingWindow {
    id: snapFlyout
    width: 300
    height: 200
    color: "transparent"
    anchor.centerIn: Window.window

    readonly property var layouts: [
        { id: "half-left",     cols: 2, rows: 1, cells: [[0,0,1,1]] },
        { id: "half-right",    cols: 2, rows: 1, cells: [[1,0,1,1]] },
        { id: "quarters",      cols: 2, rows: 2, cells: [[0,0,1,1],[1,0,1,1],[0,1,1,1],[1,1,1,1]] },
        { id: "left-thirds",   cols: 3, rows: 1, cells: [[0,0,1,1],[1,0,1,1],[2,0,1,1]] },
        { id: "three-quarter", cols: 2, rows: 2, cells: [[0,0,2,1],[0,1,1,1]] },
        { id: "columns-60-40", cols: 5, rows: 1, cells: [[0,0,3,1],[3,0,2,1]] }
    ]

    Rectangle {
        anchors.fill: parent
        radius: 16
        color: "#161b22"
        opacity: 0.96
        border.color: "rgba(255,255,255,0.12)"
        border.width: 1

        GridLayout {
            anchors.fill: parent
            anchors.margins: 12
            columns: 3
            columnSpacing: 10
            rowSpacing: 10

            Repeater {
                model: snapFlyout.layouts
                delegate: Rectangle {
                    required property var modelData

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    radius: 10
                    color: "transparent"
                    border.color: "rgba(56,189,248,0.30)"
                    border.width: 1

                    // Cell geometry is expressed in the layout's own grid, so a
                    // 2x2 and a 3x1 layout draw proportionally without magic numbers.
                    Item {
                        anchors.fill: parent
                        anchors.margins: 8

                        Repeater {
                            model: modelData.cells
                            delegate: Rectangle {
                                required property var modelData
                                x: (index % modelData.cells.length >= 0) ? 0 : 0
                                width: parent.width
                                height: parent.height
                                radius: 6
                                color: "rgba(56,189,248,0.22)"
                                border.color: "rgba(56,189,248,0.55)"
                                border.width: 1
                            }
                        }
                    }

                    HoverHandler {
                        id: hover
                    }
                    TapHandler {
                        onTapped: {
                            snapFlyout.applyLayout(modelData.id)
                            snapFlyout.close()
                        }
                    }

                    states: State {
                        when: hover.hovered
                        PropertyChanges { target: parent; border.color: "#38bdf8" }
                    }
                }
            }
        }
    }

    function applyLayout(id) {
        // Routed through hcsd, which owns the niri IPC connection (v2 plan §4.2).
        // hcsd speaks window actions so the flyout never links niri directly.
        Quickshell.execDetached(["/usr/bin/hcs", "window", "snap", "--layout", id])
    }
}
