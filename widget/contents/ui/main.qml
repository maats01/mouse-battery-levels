import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami
import "logic.js" as L

PlasmoidItem {
    id: root

    // Current view: {state, battery, charging, mode, error, lastOk}.
    property var view: ({ state: "disconnected" })
    property double now: Date.now()

    readonly property string command: '"$HOME/.local/bin/vxe" status'

    toolTipMainText: L.tooltip(view)
    Plasmoid.icon: "input-mouse"

    function refresh() {
        if (!exec.connectedSources.includes(command)) {
            exec.connectSource(command)
        }
    }

    P5Support.DataSource {
        id: exec
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => {
            disconnectSource(source)
            root.now = Date.now()
            root.view = L.merge(root.view, L.parse(data["stdout"]), root.now)
        }
    }

    Timer {
        interval: L.REFRESH_MS
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }

    onExpandedChanged: if (expanded) refresh()

    compactRepresentation: MouseArea {
        id: compact

        readonly property bool critical: L.hasLevel(root.view) && L.isCritical(root.view.battery)

        Layout.minimumWidth: row.implicitWidth
        Layout.preferredWidth: row.implicitWidth
        acceptedButtons: Qt.LeftButton
        hoverEnabled: true
        onClicked: root.expanded = !root.expanded

        RowLayout {
            id: row
            anchors.fill: parent
            spacing: Kirigami.Units.smallSpacing

            MouseIcon {
                Layout.preferredHeight: Math.min(compact.height, Kirigami.Units.iconSizes.medium)
                Layout.preferredWidth: Layout.preferredHeight * 16 / 22
                Layout.alignment: Qt.AlignVCenter
                battery: root.view.battery || 0
                charging: root.view.charging === true
                hasLevel: L.hasLevel(root.view)
                textColor: Kirigami.Theme.textColor
            }

            PlasmaComponents.Label {
                Layout.alignment: Qt.AlignVCenter
                text: L.label(root.view)
                color: compact.critical ? L.RED
                     : L.hasLevel(root.view) ? Kirigami.Theme.textColor
                     : Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.45)
                font.weight: Font.DemiBold
                font.features: { "tnum": 1 }
            }
        }
    }

    fullRepresentation: Item {
        Layout.preferredWidth: Kirigami.Units.gridUnit * 14
        Layout.preferredHeight: Kirigami.Units.gridUnit * 6
        PlasmaComponents.Label {
            anchors.centerIn: parent
            text: L.tooltip(root.view)
        }
    }
}
