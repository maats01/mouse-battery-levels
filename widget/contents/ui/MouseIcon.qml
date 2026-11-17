import QtQuick
import "logic.js" as L

// The approved "C1" mouse: thin outline, buttons split + scroll wheel, filled from
// the bottom like liquid, a small borderless bolt while charging, dashed when
// there is no level to show. Drawn in a 16x22 unit box scaled to the item height.
Canvas {
    id: root

    property int battery: 0
    property bool charging: false
    property bool hasLevel: false
    property color textColor: "white"

    readonly property color dimColor: Qt.rgba(textColor.r, textColor.g, textColor.b, 0.45)

    implicitHeight: 22
    implicitWidth: height * 16 / 22

    onBatteryChanged: requestPaint()
    onChargingChanged: requestPaint()
    onHasLevelChanged: requestPaint()
    onTextColorChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d")
        ctx.reset()
        const k = height / 22
        ctx.scale(k, k)

        // Body: rounded rect x 2.2..12.8, y 2.2..20.0, radius 5.3.
        const bx = 2.2, by = 2.2, bw = 10.6, bh = 17.8, r = 5.3
        function body() {
            ctx.beginPath()
            ctx.roundedRect(bx, by, bw, bh, r, r)
        }

        const critical = hasLevel && L.isCritical(battery)
        const stroke = !hasLevel ? dimColor : (critical ? L.RED : textColor)

        if (hasLevel && battery > 0) {
            ctx.save()
            body()
            ctx.clip()
            const h = bh * battery / 100
            ctx.fillStyle = L.levelColor(battery)
            ctx.globalAlpha = 0.9
            ctx.fillRect(bx, by + bh - h, bw, h)
            ctx.restore()
        }

        ctx.lineWidth = 1.5
        ctx.strokeStyle = stroke
        if (!hasLevel) ctx.setLineDash([1.3, 1.1])
        body()
        ctx.stroke()
        ctx.setLineDash([])

        // Button split and centre line.
        ctx.lineWidth = 1.2
        ctx.beginPath()
        ctx.moveTo(2.4, 8.6); ctx.lineTo(12.6, 8.6)
        ctx.moveTo(7.5, 2.4); ctx.lineTo(7.5, 8.6)
        ctx.stroke()

        // Scroll wheel.
        ctx.fillStyle = stroke
        ctx.beginPath()
        ctx.roundedRect(6.6, 4, 1.8, 3, 0.9, 0.9)
        ctx.fill()

        if (hasLevel && charging) {
            ctx.fillStyle = "white"
            ctx.globalAlpha = 0.92
            ctx.beginPath()
            ctx.moveTo(8.1, 11.2)
            ctx.lineTo(6.4, 14.4)
            ctx.lineTo(7.9, 14.4)
            ctx.lineTo(7.3, 17.0)
            ctx.lineTo(9.5, 13.5)
            ctx.lineTo(7.9, 13.5)
            ctx.lineTo(8.6, 11.2)
            ctx.closePath()
            ctx.fill()
            ctx.globalAlpha = 1
        }
    }
}
