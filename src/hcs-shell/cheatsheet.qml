// HCS Linux — Global Cheatsheet HUD v2.0.0
// Spec: docs/V2_STABLE_RELEASE_MASTER_PLAN.md §2
// Trigger: HCS+/ or F1 — fullscreen frosted acrylic blur:32px
//
// Every binding is written as HCS+…: the key where the Windows key sits is the
// HCS key, and it is a modifier, so no keyboard layout can move it. This file
// lists exactly the bindings in config.kdl — if they ever disagree, the
// cheatsheet is wrong and config.kdl is right.

import QtQuick
import QtQuick.Layouts
import Quickshell

PanelWindow {
    id: cheatsheet
    anchors {
        top: true; bottom: true; left: true; right: true
    }
    color: "transparent"

    readonly property var columns: [
        {
            title: "System & Window",
            height: 250,
            body: "HCS — Start Menu\nHCS+Return — Start Menu\nHCS+Space — Cycle Keyboard Layout\nHCS+/ — This Cheatsheet\nHCS+Q — Close Window\nAlt+Tab — Window Switcher\nHCS+← → — Focus Window\nHCS+Shift+← → — Move Window"
        },
        {
            title: "HCS Applications",
            height: 250,
            body: "HCS+K — HCS Chat\nHCS+I — Image Studio\nHCS+M — Monitor\nHCS+T — Terminal\nHCS+F — File Manager\nHCS+N — Notes\nHCS+Shift+S — Screenshot Region\nPrint — Screenshot Screen"
        },
        {
            title: "Desktop",
            height: 250,
            body: "HCS+E — Snap Layouts\nHCS+Tab — Task View\nHCS+Alt+S — Stage Manager\nHCS+1…4 — Virtual Desktop\nHCS+Alt+F — Floating / Tiling\nHCS+, — Settings"
        },
        {
            title: "Privacy & AI",
            height: 250,
            body: "HCS+Alt+T — Tor Killswitch\nHCS+V — Redacted Clipboard\nHCS+Alt+A — Amnesic Session\nHCS+R — Recall Timeline\nHCS+Shift+K — Ask about Selection\nHCS+Shift+F — Search the Manuals"
        }
    ]

    Rectangle {
        anchors.fill: parent
        color: "#0b0f17"
        opacity: 0.82
        ColumnLayout {
            anchors.centerIn: parent
            width: 1080
            spacing: 16
            Text { text: "⌨️  HCS Linux Hotkeys  (HCS + /)"; font.pixelSize: 22; font.bold: true; color: "#f8fafc"; Layout.alignment: Qt.AlignHCenter }
            RowLayout {
                spacing: 12
                Layout.fillWidth: true
                Repeater {
                    model: cheatsheet.columns
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        height: modelData.height
                        radius: 14
                        color: "#161b22"
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 8
                            Text { text: modelData.title; font.pixelSize: 13; font.bold: true; color: "#38bdf8" }
                            Text { text: modelData.body; font.pixelSize: 12; color: "#e2e8f0"; Layout.fillWidth: true; wrapMode: Text.Wrap }
                        }
                    }
                }
            }
            Text { text: "Keyboard: QWERTZ by default · HCS+Space cycles QWERTZ / EN-US / FR / ES / IT / GB · the HCS key itself never moves"; font.pixelSize: 11; color: "rgba(226,232,240,0.75)"; Layout.alignment: Qt.AlignHCenter }
            Text { text: "Press Esc or HCS+/ to dismiss"; font.pixelSize: 11; color: "rgba(226,232,240,0.6)"; Layout.alignment: Qt.AlignHCenter }
        }
    }
}
