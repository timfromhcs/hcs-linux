// HCS Linux — Global Cheatsheet HUD v1.0.0 Stable
// Spec: docs/V1_STABLE_RELEASE_MASTER_PLAN.md §4.4
// Trigger: Super+/ or F1 — fullscreen frosted acrylic blur:32px

import QtQuick
import QtQuick.Layouts
import Quickshell

PanelWindow {
    id: cheatsheet
    anchors {
        top: true; bottom: true; left: true; right: true
    }
    color: "transparent"

    Rectangle {
        anchors.fill: parent
        color: "#0b0f17"
        opacity: 0.82
        ColumnLayout {
            anchors.centerIn: parent
            width: 860
            spacing: 16
            Text { text: "⌨️  HCS Linux Hotkeys  (Super + /)"; font.pixelSize: 22; font.bold: true; color: "#f8fafc"; Layout.alignment: Qt.AlignHCenter }
            RowLayout {
                spacing: 12
                Layout.fillWidth: true
                // System & Window Control
                Rectangle {
                    Layout.fillWidth: true; height: 220; radius: 14; color: "#161b22"
                    Text { anchors.fill: parent; anchors.margins: 14; text: "System & Windows\n\nSuper — Start Menu\nSuper+D — Show Desktop\nSuper+Q — Close Window\nAlt+Tab — Window Switcher"; font.pixelSize: 12; color: "#e2e8f0" }
                }
                // HCS Applications
                Rectangle {
                    Layout.fillWidth: true; height: 220; radius: 14; color: "#161b22"
                    Text { anchors.fill: parent; anchors.margins: 14; text: "HCS Applications\n\nSuper+Return — HCS Chat\nSuper+Space — Search\nSuper+M — Monitor\nSuper+I — Image Studio\nSuper+T — Terminal"; font.pixelSize: 12; color: "#e2e8f0" }
                }
                // Privacy & Security
                Rectangle {
                    Layout.fillWidth: true; height: 220; radius: 14; color: "#161b22"
                    Text { anchors.fill: parent; anchors.margins: 14; text: "Privacy & Security\n\nSuper+Alt+T — Tor Killswitch\nSuper+V — Redacted Clipboard"; font.pixelSize: 12; color: "#e2e8f0" }
                }
            }
            Text { text: "Press Esc or Super+/ to dismiss"; font.pixelSize: 11; color: "rgba(226,232,240,0.6)"; Layout.alignment: Qt.AlignHCenter }
        }
    }
}
