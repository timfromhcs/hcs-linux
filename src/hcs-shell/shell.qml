// HCS Shell - Quickshell Wayland Desktop Environment
// Aesthetic: Neural Glass (Obsidian slate, glowing cyan accents, frosted translucency)
// Adheres to GEMINI.md Sections 36-60 (Zero-Electron Mandate, <= 180MB RAM)

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Wayland

ShellRoot {
    id: root

    // Top Telemetry & Control Bar
    PanelWindow {
        id: topBar
        anchors {
            top: true
            left: true
            right: true
        }
        height: 48
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            anchors.margins: 6
            radius: 12
            color: "#161b22"
            opacity: 0.88
            border.color: "rgba(255, 255, 255, 0.08)"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 16

                // Brand Emblem & Monogram
                RowLayout {
                    spacing: 8
                    Rectangle {
                        width: 24
                        height: 24
                        radius: 6
                        color: "#0f172a"
                        border.color: "#38bdf8"
                        border.width: 1
                        Text {
                            anchors.centerIn: parent
                            text: "H"
                            font.bold: true
                            font.pixelSize: 13
                            color: "#38bdf8"
                        }
                    }
                    Text {
                        text: "HCS LINUX"
                        font.bold: true
                        font.pixelSize: 13
                        font.letterSpacing: 1.5
                        color: "#f8fafc"
                    }
                }

                // Workspace Pills
                Row {
                    spacing: 6
                    Rectangle { width: 28; height: 6; radius: 3; color: "#38bdf8" }
                    Rectangle { width: 14; height: 6; radius: 3; color: "rgba(255,255,255,0.18)" }
                    Rectangle { width: 14; height: 6; radius: 3; color: "rgba(255,255,255,0.18)" }
                }

                Item { Layout.fillWidth: true }

                // Brain Telemetry Pill (Active Model & RAM)
                Rectangle {
                    width: 210
                    height: 28
                    radius: 14
                    color: "rgba(15, 23, 42, 0.75)"
                    border.color: "rgba(56, 189, 248, 0.4)"
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 8
                        Rectangle {
                            width: 8
                            height: 8
                            radius: 4
                            color: "#34d399"
                            SequentialAnimation on opacity {
                                loops: Animation.Infinite
                                NumberAnimation { from: 1.0; to: 0.4; duration: 1200 }
                                NumberAnimation { from: 0.4; to: 1.0; duration: 1200 }
                            }
                        }
                        Text {
                            text: "Qwen3-0.6B | 550 MB"
                            font.pixelSize: 11
                            font.family: "JetBrains Mono, monospace"
                            color: "#38bdf8"
                        }
                    }
                }

                // Tor Privacy Indicator
                Rectangle {
                    width: 72
                    height: 28
                    radius: 14
                    color: "rgba(49, 16, 66, 0.4)"
                    border.color: "rgba(192, 132, 252, 0.4)"
                    border.width: 1

                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 6
                        Rectangle {
                            width: 6
                            height: 6
                            radius: 3
                            color: "#c084fc"
                        }
                        Text {
                            text: "Tor"
                            font.pixelSize: 11
                            color: "#e2e8f0"
                        }
                    }
                }

                // Clock & Date
                Text {
                    text: Qt.formatDateTime(new Date(), "hh:mm")
                    font.pixelSize: 13
                    font.bold: true
                    color: "#f8fafc"
                }
            }
        }
    }

    // Glass Floating Neural Dock
    PanelWindow {
        id: dock
        anchors {
            bottom: true
            horizontalCenter: true
        }
        height: 68
        width: 440
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            anchors.bottomMargin: 8
            radius: 20
            color: "rgba(22, 27, 34, 0.88)"
            border.color: "rgba(255, 255, 255, 0.12)"
            border.width: 1

            RowLayout {
                anchors.centerIn: parent
                spacing: 16

                // App Launcher / Grid
                Rectangle {
                    width: 44; height: 44; radius: 12
                    color: "#1e293b"
                    border.color: "rgba(56, 189, 248, 0.3)"
                    Text { anchors.centerIn: parent; text: "Apps"; color: "#38bdf8"; font.pixelSize: 11; font.bold: true }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-search").start()
                    }
                }

                // HCS Chat AI Companion
                Rectangle {
                    width: 44; height: 44; radius: 12
                    color: "#1e1b4b"
                    border.color: "rgba(129, 140, 248, 0.4)"
                    Text { anchors.centerIn: parent; text: "Chat"; color: "#818cf8"; font.pixelSize: 11; font.bold: true }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-chat").start()
                    }
                }

                // HCS Search / Global Finder
                Rectangle {
                    width: 44; height: 44; radius: 12
                    color: "#064e3b"
                    border.color: "rgba(52, 211, 153, 0.4)"
                    Text { anchors.centerIn: parent; text: "Find"; color: "#34d399"; font.pixelSize: 11; font.bold: true }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-search").start()
                    }
                }

                // HCS Control Center
                Rectangle {
                    width: 44; height: 44; radius: 12
                    color: "#1e293b"
                    border.color: "rgba(56, 189, 248, 0.4)"
                    Text { anchors.centerIn: parent; text: "Ctrl"; color: "#38bdf8"; font.pixelSize: 11; font.bold: true }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-control").start()
                    }
                }

                // HCS Security Lab
                Rectangle {
                    width: 44; height: 44; radius: 12
                    color: "#311042"
                    border.color: "rgba(192, 132, 252, 0.4)"
                    Text { anchors.centerIn: parent; text: "Sec"; color: "#c084fc"; font.pixelSize: 11; font.bold: true }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("hcs-control").arg("tor").start()
                    }
                }

                // HCS Terminal
                Rectangle {
                    width: 44; height: 44; radius: 12
                    color: "#18181b"
                    border.color: "rgba(82, 82, 91, 0.5)"
                    Text { anchors.centerIn: parent; text: "Term"; color: "#f8fafc"; font.pixelSize: 11; font.bold: true }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: Quickshell.process("alacritty").start()
                    }
                }
            }
        }
    }
}
