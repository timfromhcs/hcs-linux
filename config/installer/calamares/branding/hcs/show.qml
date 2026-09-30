import QtQuick 2.15
import QtQuick.Controls 2.15

// HCS Calamares branded slideshow — Silicon Valley Neural Glass
Rectangle {
    anchors.fill: parent
    color: "#0b0f17"
    Column {
        anchors.centerIn: parent
        spacing: 12
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "HCS LINUX v1.0.0"; font.pixelSize: 28; font.bold: true; color: "#38bdf8" }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Local-First AI OS — Neural Glass 3.0"; font.pixelSize: 14; color: "#e2e8f0" }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "• Offline CPU Image Studio  • Security Lab  • Tor Shield  • Dev Suite"; font.pixelSize: 12; color: "#94a3b8" }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: "SquashFS extraction in progress — LUKS2 encryption optional on next screen."; font.pixelSize: 11; color: "#64748b" }
    }
}
