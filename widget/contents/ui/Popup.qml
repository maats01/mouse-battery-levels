import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.plasma.components as PlasmaComponents
import org.kde.kirigami as Kirigami
import "logic.js" as L

// Approved popup "B" with the DPI slider "S3" (monochrome, dots at the other stages).
Item {
    id: popup

    property var view
    property bool busy: false
    property string message: ""

    signal setDpi(int dpi)

    readonly property int currentDpi: view.dpi !== undefined ? view.dpi : 0
    readonly property bool canEdit: L.canSetDpi(view, busy) && view.dpi !== undefined

    // Keep slider and field on the mouse's value unless the user is interacting.
    function syncDpi() {
        if (!slider.pressed) slider.value = popup.currentDpi
        if (!dpiField.visible) dpiField.text = String(popup.currentDpi)
    }
    onViewChanged: syncDpi()
    onBusyChanged: if (!busy) syncDpi()

    function applyDpi(n) {
        if (n !== null && n !== popup.currentDpi) popup.setDpi(n)
        else syncDpi()
    }

    readonly property bool hasLevel: L.hasLevel(view)
    readonly property color dim: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                                         Kirigami.Theme.textColor.b, 0.6)

    // Plasma 6 remembers popup sizes; a minimum makes it grow to fit the content.
    Layout.minimumWidth: Kirigami.Units.gridUnit * 16
    Layout.preferredWidth: Kirigami.Units.gridUnit * 16
    Layout.minimumHeight: col.implicitHeight + Kirigami.Units.largeSpacing * 2
    Layout.preferredHeight: Layout.minimumHeight

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
            Layout.topMargin: Kirigami.Units.largeSpacing

            PlasmaComponents.Label {
                text: "DPI"
                color: popup.dim
                font.pointSize: Kirigami.Theme.smallFont.pointSize
                font.capitalization: Font.AllUppercase
                font.letterSpacing: 1
            }

            PlasmaComponents.Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignRight
                text: popup.busy ? "aplicando…" : popup.message
                color: popup.busy ? popup.dim : L.RED
                font: Kirigami.Theme.smallFont
                elide: Text.ElideRight
            }

            // Click the number to type a value; Enter applies, Esc cancels.
            PlasmaComponents.Label {
                id: dpiLabel
                visible: !dpiField.visible
                text: slider.pressed ? Math.round(slider.value) : (popup.view.dpi !== undefined ? popup.view.dpi : "––")
                font.weight: Font.DemiBold
                font.features: { "tnum": 1 }
                opacity: popup.canEdit ? 1 : 0.6
                MouseArea {
                    anchors.fill: parent
                    enabled: popup.canEdit
                    cursorShape: Qt.IBeamCursor
                    onClicked: {
                        dpiField.text = String(popup.currentDpi)
                        dpiField.visible = true
                        dpiField.forceActiveFocus()
                        dpiField.selectAll()
                    }
                }
            }

            PlasmaComponents.TextField {
                id: dpiField
                visible: false
                Layout.preferredWidth: Kirigami.Units.gridUnit * 4
                horizontalAlignment: TextInput.AlignRight
                inputMethodHints: Qt.ImhDigitsOnly
                validator: RegularExpressionValidator { regularExpression: /[0-9]{0,5}/ }
                font.features: { "tnum": 1 }
                onAccepted: {
                    visible = false
                    popup.applyDpi(L.normalizeDpi(text))
                }
                Keys.onEscapePressed: { visible = false; popup.syncDpi() }
                onActiveFocusChanged: if (!activeFocus && visible) { visible = false; popup.syncDpi() }
            }
        }

        T.Slider {
            id: slider
            visible: popup.hasLevel
            enabled: popup.canEdit
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.smallSpacing
            from: L.SLIDER_MIN
            to: L.SLIDER_MAX
            stepSize: L.SLIDER_STEP
            snapMode: T.Slider.SnapAlways
            implicitHeight: 18
            Component.onCompleted: value = popup.currentDpi
            onPressedChanged: if (!pressed) popup.applyDpi(Math.round(value))

            readonly property color fg: Kirigami.Theme.textColor

            background: Item {
                x: slider.leftPadding
                y: slider.topPadding + slider.availableHeight / 2 - 2
                width: slider.availableWidth
                height: 4
                Rectangle {
                    anchors.fill: parent
                    radius: 2
                    color: Qt.rgba(slider.fg.r, slider.fg.g, slider.fg.b, 0.15)
                }
                Rectangle {
                    width: 8 + slider.visualPosition * (parent.width - 16)
                    height: parent.height
                    radius: 2
                    color: Qt.rgba(slider.fg.r, slider.fg.g, slider.fg.b, 0.85)
                }
                Repeater {
                    model: L.otherStages(popup.view)
                    Rectangle {
                        x: 8 + L.sliderFraction(modelData) * (parent.width - 16) - 2
                        y: 0
                        width: 4
                        height: 4
                        radius: 2
                        color: Qt.rgba(slider.fg.r, slider.fg.g, slider.fg.b, 0.45)
                    }
                }
            }

            handle: Rectangle {
                x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                width: 16
                height: 16
                radius: 8
                color: slider.fg
                border.width: 3
                border.color: Qt.rgba(slider.fg.r, slider.fg.g, slider.fg.b, 0.12)
            }
        }

        Item {
            visible: popup.hasLevel
            Layout.fillWidth: true
            implicitHeight: Kirigami.Units.gridUnit
            Repeater {
                model: L.otherStages(popup.view)
                PlasmaComponents.Label {
                    text: modelData
                    color: popup.dim
                    font.pointSize: Kirigami.Theme.smallFont.pointSize * 0.9
                    font.features: { "tnum": 1 }
                    x: Math.max(0, Math.min(parent.width - width,
                        slider.leftPadding + L.sliderFraction(modelData) * (slider.availableWidth - 16) + 8 - width / 2))
                }
            }
        }
    }
}
