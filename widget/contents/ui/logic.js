.pragma library

// Pure helpers for the VXE R1 widget. No QML types here, so tests can import it.

var REFRESH_MS = 60000

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
            mode: o.mode === "wired" ? "wired" : "wireless"
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
    if (cur.state === "disconnected" && prev && hasLevel(prev)) {
        return {
            state: "asleep",
            battery: prev.battery,
            charging: false,
            mode: prev.mode,
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
    return "Erro ao ler o mouse"
}

// A sleeping mouse is shown exactly like an awake one (user's call).
function connectionText(v) {
    if (!hasLevel(v)) return "desconectado"
    if (v.mode === "wired") return v.charging ? "cabo · carregando" : "cabo"
    return "2.4 GHz · conectado"
}

function stateText(v) {
    if (!hasLevel(v)) return ""
    return v.charging ? "carregando" : "descarregando"
}
