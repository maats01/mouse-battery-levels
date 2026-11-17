.pragma library

// Pure helpers for the VXE R1 widget. No QML types here, so tests can import it.

var REFRESH_MS = 60000
var RETRY_MS = 3000

var GREEN = "#3ccf7a"
var YELLOW = "#f0b429"
var RED = "#f0525f"

function clamp(n) {
    return Math.max(0, Math.min(100, Math.round(n)))
}

// One `vxe status` stdout line -> {state, battery, charging, mode, error}.
function parse(stdout) {
    var o
    try {
        o = JSON.parse(String(stdout).trim())
    } catch (e) {
        o = null
    }
    if (o === null || typeof o !== "object") {
        return { state: "error", error: "cli" }
    }
    if (typeof o.error === "string") {
        return { state: "error", error: o.error }
    }
    if (o.connected === false) {
        return { state: "disconnected" }
    }
    if (o.connected === true && typeof o.battery === "number") {
        return {
            state: "ok",
            battery: clamp(o.battery),
            charging: o.charging === true,
            mode: o.mode === "wired" ? "wired" : "wireless",
            dpi: typeof o.dpi === "number" ? o.dpi : undefined,
            stages: validStages(o.stages)
        }
    }
    return { state: "error", error: "cli" }
}

// Combine the previous view with a new reading. A sleeping mouse keeps the
// last known level (state "asleep"); errors drop it.
function merge(prev, cur, now) {
    if (cur.state === "ok") {
        return Object.assign({}, cur, { lastOk: now })
    }
    var transient = cur.state === "disconnected"
        || (cur.state === "error" && (cur.error === "io" || cur.error === "cli"))
    if (transient && prev && hasLevel(prev)) {
        return {
            state: "asleep",
            battery: prev.battery,
            charging: false,
            mode: prev.mode,
            dpi: prev.dpi,
            stages: prev.stages,
            lastOk: prev.lastOk
        }
    }
    return cur
}

function hasLevel(v) {
    return v.state === "ok" || v.state === "asleep"
}

function levelColor(battery) {
    if (battery >= 40) return GREEN
    if (battery >= 15) return YELLOW
    return RED
}

function isCritical(battery) {
    return battery < 15
}

function label(v) {
    return hasLevel(v) ? v.battery + "%" : "––"
}

function tooltip(v) {
    switch (v.state) {
    case "ok":
        return "VXE R1 — " + v.battery + "%" + (v.charging ? " · carregando" : "")
    case "asleep":
        return "VXE R1 — " + v.battery + "%"
    case "disconnected":
        return "Mouse dormindo ou desligado"
    }
    if (v.error === "no_receiver") return "Receptor não encontrado"
    if (v.error === "permission") return "Sem permissão — rode install.sh"
    if (v.error === "write_failed") return "Não foi possível mudar o DPI"
    return "Erro ao ler o mouse"
}



function connectionText(v) {
    return v.mode === "wired" ? "cabo" : "2.4 GHz"
}

// After an error (e.g. cable moving between receiver and mouse) retry soon.
function nextDelay(v) {
    return v.state === "error" ? RETRY_MS : REFRESH_MS
}

var DPI_MIN = 50
var DPI_MAX = 10000

// Typed text -> DPI the sensor accepts (multiple of 50 in 50..10000), or null.
function normalizeDpi(text) {
    var t = String(text).trim()
    if (!/^[0-9]+$/.test(t)) return null
    var n = Math.round(parseInt(t, 10) / 50) * 50
    return Math.max(DPI_MIN, Math.min(DPI_MAX, n))
}

function dpiCommand(dpi) {
    if (dpi === null || normalizeDpi(String(dpi)) !== dpi) return ""
    return '"$HOME/.local/bin/vxe" dpi ' + dpi
}

function canSetDpi(v, busy) {
    return !busy && hasLevel(v)
}

// Message under the DPI field after a `vxe dpi` run ("" on success).
function dpiResultMessage(cur) {
    if (cur.state === "ok") return ""
    if (cur.state === "disconnected") return "Mexa o mouse e tente de novo"
    return tooltip(cur)
}

function validStages(a) {
    if (!Array.isArray(a)) return []
    for (var i = 0; i < a.length; i++) {
        if (typeof a[i] !== "number") return []
    }
    return a.slice()
}

// Slider covers the user's stages; typed values may go beyond (50..10000).
var SLIDER_MIN = 400
var SLIDER_MAX = 3200
var SLIDER_STEP = 50

function sliderFraction(dpi) {
    var f = (dpi - SLIDER_MIN) / (SLIDER_MAX - SLIDER_MIN)
    return Math.max(0, Math.min(1, f))
}

// Stages other than the active one (one match of the current DPI removed),
// limited to the slider range: drawn as dots on the track.
function otherStages(v) {
    var st = (v.stages || []).slice()
    var i = st.indexOf(v.dpi)
    if (i >= 0) st.splice(i, 1)
    return st.filter(function (d) { return d >= SLIDER_MIN && d <= SLIDER_MAX })
}
