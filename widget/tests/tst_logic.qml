import QtQuick
import QtTest
import "../contents/ui/logic.js" as L

TestCase {
    name: "Logic"

    function test_parse_ok() {
        const s = L.parse('{"connected":true,"mode":"wireless","battery":82,"charging":false}\n')
        compare(s.state, "ok")
        compare(s.battery, 82)
        compare(s.charging, false)
        compare(s.mode, "wireless")
    }

    function test_parse_disconnected() {
        compare(L.parse('{"connected":false}').state, "disconnected")
    }

    function test_parse_error_permission() {
        const s = L.parse('{"error":"permission"}')
        compare(s.state, "error")
        compare(s.error, "permission")
    }

    function test_parse_garbage() {
        for (const g of ["", "oops", "{", "null", "42"]) {
            const s = L.parse(g)
            compare(s.state, "error", g)
            compare(s.error, "cli", g)
        }
    }

    function test_parse_clamps_battery() {
        compare(L.parse('{"connected":true,"mode":"wired","battery":140,"charging":true}').battery, 100)
        compare(L.parse('{"connected":true,"mode":"wired","battery":-3,"charging":true}').battery, 0)
    }

    function test_merge_ok_sets_last_ok() {
        const v = L.merge(null, L.parse('{"connected":true,"mode":"wireless","battery":82,"charging":false}'), 1000)
        compare(v.state, "ok")
        compare(v.lastOk, 1000)
    }

    function test_merge_asleep_keeps_last_battery() {
        const prev = L.merge(null, L.parse('{"connected":true,"mode":"wireless","battery":82,"charging":false}'), 1000)
        const v = L.merge(prev, L.parse('{"connected":false}'), 61000)
        compare(v.state, "asleep")
        compare(v.battery, 82)
        compare(v.charging, false)
        compare(v.lastOk, 1000)
    }

    function test_merge_disconnected_without_history() {
        compare(L.merge(null, L.parse('{"connected":false}'), 5).state, "disconnected")
    }

    function test_merge_error_drops_history() {
        const prev = L.merge(null, L.parse('{"connected":true,"mode":"wireless","battery":82,"charging":false}'), 1000)
        compare(L.merge(prev, L.parse('{"error":"no_receiver"}'), 2000).state, "error")
    }

    function test_merge_transient_error_keeps_level() {
        const prev = L.merge(null, L.parse('{"connected":true,"mode":"wireless","battery":82,"charging":false}'), 1000)
        for (const raw of ['{"error":"io"}', "", "oops"]) {
            const v = L.merge(prev, L.parse(raw), 2000)
            compare(v.state, "asleep", raw)
            compare(v.battery, 82, raw)
        }
    }

    function test_next_delay_retries_fast_after_error() {
        compare(L.nextDelay({state: "error", error: "no_receiver"}), 3000)
        compare(L.nextDelay({state: "error", error: "io"}), 3000)
        compare(L.nextDelay({state: "ok", battery: 50}), 60000)
        compare(L.nextDelay({state: "asleep", battery: 50}), 60000)
        compare(L.nextDelay({state: "disconnected"}), 60000)
    }

    function test_level_color() {
        compare(L.levelColor(100), "#3ccf7a")
        compare(L.levelColor(40), "#3ccf7a")
        compare(L.levelColor(39), "#f0b429")
        compare(L.levelColor(15), "#f0b429")
        compare(L.levelColor(14), "#f0525f")
        compare(L.levelColor(0), "#f0525f")
    }

    function test_is_critical() {
        compare(L.isCritical(14), true)
        compare(L.isCritical(15), false)
    }

    function test_has_level() {
        compare(L.hasLevel({state: "ok"}), true)
        compare(L.hasLevel({state: "asleep"}), true)
        compare(L.hasLevel({state: "disconnected"}), false)
        compare(L.hasLevel({state: "error"}), false)
    }

    function test_label() {
        compare(L.label({state: "ok", battery: 82}), "82%")
        compare(L.label({state: "asleep", battery: 9}), "9%")
        compare(L.label({state: "disconnected"}), "––")
        compare(L.label({state: "error", error: "cli"}), "––")
    }

    function test_tooltip() {
        compare(L.tooltip({state: "ok", battery: 82, charging: false}), "VXE R1 — 82%")
        compare(L.tooltip({state: "asleep", battery: 82, charging: false}), "VXE R1 — 82%")
        compare(L.tooltip({state: "disconnected"}), "Mouse dormindo ou desligado")
        compare(L.tooltip({state: "error", error: "no_receiver"}), "Receptor não encontrado")
        compare(L.tooltip({state: "error", error: "io"}), "Erro ao ler o mouse")
        compare(L.tooltip({state: "error", error: "cli"}), "Erro ao ler o mouse")
    }

    function test_tooltip_permission() {
        compare(L.tooltip({state: "error", error: "permission"}), "Sem permissão — rode install.sh")
    }

    function test_tooltip_charging() {
        compare(L.tooltip({state: "ok", battery: 50, charging: true}), "VXE R1 — 50% · carregando")
    }

    function test_connection_text() {
        compare(L.connectionText({state: "ok", mode: "wireless", charging: false}), "2.4 GHz")
        compare(L.connectionText({state: "asleep", mode: "wireless", charging: false}), "2.4 GHz")
        compare(L.connectionText({state: "ok", mode: "wired", charging: true}), "cabo")
        compare(L.connectionText({state: "ok", mode: "wired", charging: false}), "cabo")
    }

    function test_state_text() {
        compare(L.stateText({state: "ok", charging: true}), "carregando")
        compare(L.stateText({state: "ok", charging: false}), "descarregando")
        compare(L.stateText({state: "asleep", charging: false}), "descarregando")
        compare(L.stateText({state: "error"}), "")
    }

    function test_refresh_ms() {
        compare(L.REFRESH_MS, 60000)
    }
}
