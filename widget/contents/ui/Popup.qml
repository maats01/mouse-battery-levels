import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents
import org.kde.kirigami as Kirigami
import "logic.js" as L

// Approved popup "B" (phase 1: battery only, no DPI section).
Item {
    id: popup

    property var view
    property bool busy: false
    property string message: ""

    signal setDpi(int dpi)

    function syncDpiField() {
        dpiField.text = popup.view.dpi !== undefined ? String(popup.view.dpi) : ""
    }
    onViewChanged: if (!dpiField.activeFocus) syncDpiField()
    onBusyChanged: if (!busy) syncDpiField()

    readonly property bool hasLevel: L.hasLevel(view)
    readonly property color dim: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                                         Kirigami.Theme.textColor.b, 0.6)

    Layout.preferredWidth: Kirigami.Units.gridUnit * 16
    Layout.preferredHeight: col.implicitHeight + Kirigami.Units.largeSpacing * 2

    ColumnLayout {
        id: col
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            spacing: Kirigami.Units.largeSpacing

            MouseIcon {
                Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                Layout.preferredWidth: Layout.preferredHeight * 16 / 22
                battery: popup.view.battery || 0
                charging: popup.view.charging === true
                hasLevel: popup.hasLevel
                textColor: Kirigami.Theme.textColor
            }

            ColumnLayout {
                spacing: 0
                PlasmaComponents.Label {
                    text: "VXE R1"
                    font.weight: Font.DemiBold
                }
                PlasmaComponents.Label {
                    text: popup.hasLevel ? L.connectionText(popup.view) : L.tooltip(popup.view)
                    color: popup.dim
                    font: Kirigami.Theme.smallFont
                }
            }

            Item { Layout.fillWidth: true }

            RowLayout {
                visible: popup.hasLevel
                spacing: 1
                PlasmaComponents.Label {
                    Layout.alignment: Qt.AlignBaseline
                    text: popup.view.battery !== undefined ? popup.view.battery : ""
                    font.pixelSize: Kirigami.Units.gridUnit * 1.8
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
                PlasmaComponents.Label {
                    Layout.alignment: Qt.AlignBaseline
                    text: "%"
                    color: popup.dim
                    font.pixelSize: Kirigami.Units.gridUnit
                }
            }
        }

        Rectangle {
            visible: popup.hasLevel
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.smallSpacing
            implicitHeight: 6
            radius: 3
            color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                           Kirigami.Theme.textColor.b, 0.15)
            Rectangle {
                width: parent.width * (popup.view.battery || 0) / 100
                height: parent.height
                radius: 3
                color: L.levelColor(popup.view.battery || 0)
            }
        }

        RowLayout {
            visible: popup.hasLevel
            Layout.fillWidth: true
            PlasmaComponents.Label {
                text: L.stateText(popup.view)
                color: popup.dim
                font: Kirigami.Theme.smallFont
            }
        }

        PlasmaComponents.Label {
            visible: popup.hasLevel
            Layout.topMargin: Kirigami.Units.largeSpacing
            text: "DPI"
            color: popup.dim
            font.pointSize: Kirigami.Theme.smallFont.pointSize
            font.capitalization: Font.AllUppercase
            font.letterSpacing: 1
        }

        RowLayout {
            visible: popup.hasLevel
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            PlasmaComponents.TextField {
                id: dpiField
                Layout.preferredWidth: Kirigami.Units.gridUnit * 5
                enabled: L.canSetDpi(popup.view, popup.busy)
                horizontalAlignment: TextInput.AlignHCenter
                inputMethodHints: Qt.ImhDigitsOnly
                validator: RegularExpressionValidator { regularExpression: /[0-9]{0,5}/ }
                font.features: { "tnum": 1 }
                Component.onCompleted: popup.syncDpiField()
                onAccepted: {
                    const n = L.normalizeDpi(text)
                    if (n === null) {
                        popup.syncDpiField()
                    } else {
                        text = String(n)
                        popup.setDpi(n)
                    }
                }
            }

            PlasmaComponents.Label {
                Layout.fillWidth: true
                text: popup.busy ? "aplicando…" : popup.message
                color: popup.busy ? popup.dim : L.RED
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
            }
        }
    }
}
