import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami
import "logic.js" as L

PlasmoidItem {
    id: root

    // Current view: {state, battery, charging, mode, error, lastOk}.
    property var view: ({ state: "disconnected" })
    property double now: Date.now()
    property bool dpiBusy: false
    property string dpiMessage: ""

    readonly property string command: '"$HOME/.local/bin/vxe" status'

    toolTipMainText: L.tooltip(view)
    Plasmoid.icon: "input-mouse"

    function setDpi(n) {
        const cmd = L.dpiCommand(n)
        if (!L.canSetDpi(view, dpiBusy) || cmd === "") return
        dpiBusy = true
        dpiMessage = ""
        exec.connectSource(cmd)
    }

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
            const cur = L.parse(data["stdout"])
            if (source !== root.command) {
                // Result of `vxe dpi N`.
                // Update the view before clearing busy: the popup re-syncs its
                // DPI field when busy goes false.
                if (cur.state === "ok") root.view = L.merge(root.view, cur, root.now)
                root.dpiMessage = L.dpiResultMessage(cur)
                root.dpiBusy = false
                return
            }
            root.view = L.merge(root.view, cur, root.now)
            poll.interval = L.nextDelay(cur)
            poll.restart()
        }
    }

    Timer {
        id: poll
        interval: L.REFRESH_MS
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }

    onExpandedChanged: () => { if (root.expanded) root.refresh() }

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

    fullRepresentation: Popup {
        view: root.view
        busy: root.dpiBusy
        message: root.dpiMessage
        onSetDpi: dpi => root.setDpi(dpi)
    }
}
