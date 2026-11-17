# VXE R1 na bandeja — Fase 1 (bateria) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mostrar a bateria do VXE R1 (receptor e cabo) num widget do Plasma 6 com o ícone “C1” e o popup “B” aprovados, sem processo residente e sem acordar o mouse à toa.

**Architecture:** Um binário Rust `vxe` (sem dependências externas) encontra o hidraw do mouse via sysfs, faz uma troca de report, imprime uma linha JSON e sai. Um plasmoid QML chama `vxe status` pelo data engine `executable` do `plasma5support` ao abrir o popup e num intervalo fixo, e desenha ícone/popup. Uma regra udev `uaccess` dá acesso sem sudo.

**Tech Stack:** Rust 1.99 (std only), QML / Plasma 6.8 (`org.kde.plasma.plasmoid`, `org.kde.plasma.plasma5support`, `org.kde.kirigami`), `qmltestrunner` para a lógica JS, udev.

**Spec:** `docs/superpowers/specs/2026-11-17-vxe-r1-tray-design.md`

**Fora deste plano:** Fase 2 (DPI). Ganha plano próprio depois que a Task 2 documentar o protocolo de DPI. O popup da fase 1 não mostra a seção DPI.

## Global Constraints

- Receptor: `HID_ID=0003:0000373B:000010C9`; ID com fio: o que a Task 2 registrar em `docs/protocol.md`.
- CLI: só `std`, sem crates. Saída é **uma linha JSON** em stdout; erro → código de saída ≠ 0.
- Timeout por requisição HID: 300 ms. `vxe status` < 50 ms no caso normal.
- Nada residente: a CLI abre o hidraw, troca o report e **fecha**.
- Limites de cor: verde ≥ 40%, amarelo 15–39%, vermelho < 15% (abaixo de 15% contorno e número também vermelhos). Cores: verde `#3ccf7a`, amarelo `#f0b429`, vermelho `#f0525f`.
- Carregando: mesma cor do nível + raio pequeno branco sem contorno no corpo do mouse. Sem cor especial.
- Desconectado/erro: contorno tracejado cinza, texto `––`.
- Visual clean: traço do mouse ~1.5 px, número semibold com algarismos tabulares. Nada de bold extra.
- Intervalo de atualização: 60 s se a Task 2 concluir que a consulta não acorda o mouse; senão 300 s. Valor fica em `REFRESH_MS` (Task 6).
- Mockups de referência: `.superpowers/brainstorm/7930-1794927771/content/polish-v1.html` e `charging-v3.html`.
- Textos de interface em português.

## Review Focus

1. **Mouse dormindo durante a consulta.** O receptor responde “sem mouse” ou não responde. Esperado: `{"connected":false}`, ícone tracejado, sem erro piscando. Teste: Task 3 (`parse` de fixture “dormindo”) e Task 5 (timeout → `connected:false`).
2. **Receptor e cabo plugados ao mesmo tempo.** Dois dispositivos casam. Esperado: usar o que a Task 2 apontou como fonte confiável de bateria/carregando. Teste: Task 4 `find_prefers_wired_when_both_present`.
3. **Hidraw sem permissão** (regra udev ausente ou sessão antiga). Esperado: `{"error":"permission"}`, tooltip “Sem permissão — rode install.sh”. Teste: Task 5 mapeia `PermissionDenied`; Task 6 `test_tooltip_permission`.
4. **CLI ausente ou falhando** (binário fora do PATH, saída vazia ou JSON inválido). Esperado: o widget mostra o estado de erro e não quebra. Teste: Task 6 `test_parse_garbage`.
5. **Valores de bateria fora da faixa** (>100, negativos, ou um byte cru estranho). Esperado: limitar a 0–100. Teste: Task 3 `parse_clamps_battery`.

---

## File Structure

```
.gitignore                     target/, .superpowers/
udev/70-vxe-r1.rules           regra uaccess (receptor + com fio)
docs/protocol.md               resultado do spike (bytes, layout, IDs, wake)
spike/probe.py                 script descartável do spike (stdlib)
cli/Cargo.toml
cli/src/main.rs                argumentos, orquestra, códigos de saída
cli/src/protocol.rs            montar request / parsear resposta (puro)
cli/src/status.rs              struct Status + to_json (puro)
cli/src/device.rs              achar hidraw no sysfs (puro sobre um root)
cli/src/hidraw.rs              open/write/poll/read com timeout, close
cli/tests/fixtures/*.hex       respostas cruas capturadas na Task 2
widget/metadata.json
widget/contents/ui/main.qml    data engine, timer, representações
widget/contents/ui/MouseIcon.qml   ícone C1 (Canvas/Shapes)
widget/contents/ui/Popup.qml   popup B
widget/contents/ui/logic.js    funções puras: parse, cor, estado, textos
widget/tests/tst_logic.qml     qmltestrunner
install.sh
```

---

### Task 1: Repositório e permissão do receptor

**Files:**
- Create: `.gitignore`, `udev/70-vxe-r1.rules`

- [ ] **Step 1:** `git init` na raiz; `.gitignore` com `target/` e `.superpowers/`. Commit inicial com `docs/`.
- [ ] **Step 2:** Criar `udev/70-vxe-r1.rules`:
  ```
  SUBSYSTEM=="hidraw", ATTRS{idVendor}=="373b", ATTRS{idProduct}=="10c9", TAG+="uaccess"
  ```
- [ ] **Step 3:** **O usuário roda** (passo com sudo): `sudo cp udev/70-vxe-r1.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger`
- [ ] **Step 4: Verificar.** `getfacl /dev/hidraw0 /dev/hidraw1 /dev/hidraw2` → cada um lista `user:mats:rw-`.
- [ ] **Step 5:** Commit `chore: init repo and udev rule for receiver`.

### Task 2: Spike de protocolo (hardware real, interativo)

Descartável. Precisa do usuário para plugar/desplugar o cabo e deixar o mouse dormir. A saída é `docs/protocol.md` + fixtures, que as Tasks 3–5 usam como fonte da verdade.

**Files:**
- Create: `spike/probe.py`, `docs/protocol.md`, `cli/tests/fixtures/*.hex`

- [ ] **Step 1: Levantar referências.** Ler o código de bateria de [MouseBattery](https://github.com/zhijuyingfeng/MouseBattery), [atk-control](https://github.com/NasirA1991/atk-control), [OpenVXE](https://github.com/FrancescoLFM/OpenVXE) e `drivers/hid/hid-kysona.c`. Anotar os candidatos: report ID, tamanho, bytes do comando, offset da bateria/carregando e qual interface (pelo report descriptor em `/sys/class/hidraw/hidrawN/device/report_descriptor`).
- [ ] **Step 2: Sondar.** `spike/probe.py <hidrawN> <hex-request>` escreve o request, faz `select` com timeout de 300 ms, imprime a resposta em hex e o tempo. Rodar os candidatos nas 3 interfaces até a resposta mudar com a bateria (comparar com o web driver oficial).
- [ ] **Step 3: Cabo.** Com o cabo plugado: `lsusb` (anotar o ID com fio, se aparecer um novo), repetir a sonda nos dois caminhos e anotar onde aparece “carregando” e se a % continua confiável.
- [ ] **Step 4: Repouso.** Deixar o mouse parado até dormir (LED/tempo do fabricante), sondar uma vez e anotar: (a) a resposta (vem do cache? é “sem mouse”?); (b) se o mouse acordou. Repetir 3×. Decidir `REFRESH_MS`: 60000 se não acorda, 300000 se acorda.
- [ ] **Step 5: DPI (só documentar, não implementar).** Procurar comando de ativar estágio / definir DPI e anotar o que existir. Não gravar nada de config.
- [ ] **Step 6: Escrever `docs/protocol.md`** com: interface escolhida por caminho (receptor/cabo) e como identificá-la; bytes do request; layout da resposta (offsets de bateria, carregando, marcador de “mouse ausente”); ID com fio; resultado do teste de repouso e o `REFRESH_MS` escolhido; notas de DPI.
- [ ] **Step 7: Salvar fixtures** (hex de uma linha cada) em `cli/tests/fixtures/`: `wireless_82.hex` (ou o nível real), `wireless_sleeping.hex`, `wired_charging.hex`.
- [ ] **Step 8:** Se o ID com fio existir, acrescentar uma linha igual em `udev/70-vxe-r1.rules` e pedir ao usuário que reinstale (Task 1 Step 3).
- [ ] **Step 9:** Commit `docs: document VXE R1 battery protocol from spike`.

**Gate:** se não for possível ler a bateria por nenhum caminho, parar e trazer ao usuário o plano B (gravar o tráfego do web driver oficial com `usbmon`/Wireshark).

### Task 3: `protocol` e `status` (puros)

**Files:**
- Create: `cli/Cargo.toml` (package `vxe`, edition 2024, sem dependências), `cli/src/protocol.rs`, `cli/src/status.rs`, `cli/src/main.rs` (só `mod` por enquanto)
- Test: testes unitários nos próprios módulos, usando `include_str!("../tests/fixtures/...")`

**Interfaces:**
- Produces:
  - `pub enum Path { Wireless, Wired }`
  - `pub fn battery_request(path: Path) -> Vec<u8>`: bytes exatos de `docs/protocol.md`.
  - `pub enum Reading { Present { battery: u8, charging: bool }, Absent }`
  - `pub fn parse_battery(path: Path, resp: &[u8]) -> Result<Reading, ParseError>`, com `pub struct ParseError(pub String)`
  - `pub enum Status { Ok { mode: Path, battery: u8, charging: bool }, Disconnected, Error(ErrKind) }` e `pub enum ErrKind { NoReceiver, Permission, Io }`
  - `impl Status { pub fn to_json(&self) -> String; pub fn exit_code(&self) -> i32 }`

- [ ] **Step 1: Escrever os testes que falham.**
  ```rust
  #[test] fn parse_wireless_fixture() {
      let r = parse_battery(Path::Wireless, &hex(include_str!("../tests/fixtures/wireless_82.hex"))).unwrap();
      assert_eq!(r, Reading::Present { battery: 82, charging: false }); // valor real do fixture
  }
  #[test] fn parse_sleeping_fixture() { /* wireless_sleeping.hex -> Reading::Absent */ }
  #[test] fn parse_wired_charging() { /* wired_charging.hex -> Present { charging: true, .. } */ }
  #[test] fn parse_rejects_wrong_report_id() { assert!(parse_battery(Path::Wireless, &[0x00; 16]).is_err()); }
  #[test] fn parse_rejects_short() { assert!(parse_battery(Path::Wireless, &[0x08]).is_err()); }
  #[test] fn parse_clamps_battery() { /* fixture com o byte de bateria trocado por 0xFF -> battery == 100 */ }
  #[test] fn json_ok() {
      assert_eq!(Status::Ok { mode: Path::Wireless, battery: 82, charging: false }.to_json(),
                 r#"{"connected":true,"mode":"wireless","battery":82,"charging":false}"#);
  }
  #[test] fn json_disconnected() { assert_eq!(Status::Disconnected.to_json(), r#"{"connected":false}"#); }
  #[test] fn json_errors() {
      assert_eq!(Status::Error(ErrKind::NoReceiver).to_json(), r#"{"error":"no_receiver"}"#);
      assert_eq!(Status::Error(ErrKind::Permission).to_json(), r#"{"error":"permission"}"#);
      assert_eq!(Status::Error(ErrKind::Io).to_json(), r#"{"error":"io"}"#);
  }
  #[test] fn exit_codes() { /* Ok e Disconnected -> 0; Error(_) -> 1 */ }
  ```
  (`hex()` é um helper de teste que converte a string hex em `Vec<u8>`.)
- [ ] **Step 2:** `cd cli && cargo test`. Esperado: FAIL (símbolos não existem).
- [ ] **Step 3:** Implementar `protocol.rs` e `status.rs` com as assinaturas acima. JSON escrito à mão (só números, bools e strings fixas).
- [ ] **Step 4:** `cargo test`. Esperado: todos passam.
- [ ] **Step 5:** Commit `feat(cli): battery protocol parsing and status JSON`.

### Task 4: `device`: achar o hidraw

**Files:**
- Create: `cli/src/device.rs`

**Interfaces:**
- Consumes: `Path` (Task 3)
- Produces: `pub fn find(sysfs_hidraw: &std::path::Path) -> Option<(Path, std::path::PathBuf)>`: devolve o caminho (`Wireless`/`Wired`) e `/dev/hidrawN`. Casa `HID_ID` em `<root>/hidrawN/device/uevent` e escolhe a interface pela regra de `docs/protocol.md`. Se os dois casarem, prefere o caminho que o protocolo apontou como fonte confiável com cabo.

- [ ] **Step 1: Testes que falham**, com uma árvore sysfs falsa em `std::env::temp_dir()` (diretório por teste):
  - `find_none_when_no_match`: só um hidraw de outro vendor → `None`
  - `find_receiver_interface`: três hidraw `373B:10C9` → devolve o da interface de config com `Path::Wireless`
  - `find_prefers_wired_when_both_present`: receptor + com fio → o caminho definido no protocolo
  - `find_case_insensitive_hid_id`: `HID_ID` em minúsculas também casa
- [ ] **Step 2:** `cargo test device`. Esperado: FAIL.
- [ ] **Step 3:** Implementar `find`.
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5:** Commit `feat(cli): locate VXE R1 hidraw via sysfs`.

### Task 5: `hidraw` + `main`: `vxe status` de verdade

**Files:**
- Create: `cli/src/hidraw.rs`
- Modify: `cli/src/main.rs`

**Interfaces:**
- Consumes: `find`, `battery_request`, `parse_battery`, `Status`
- Produces: o binário `vxe` com o subcomando `status` (outro argumento → uso no stderr, saída 2). `pub fn exchange(dev: &Path, req: &[u8], timeout_ms: u16) -> io::Result<Option<Vec<u8>>>`: `None` em timeout. Abre, escreve, faz `poll` via `libc`-free `std::os::fd` + syscall `poll` declarada com `extern "C"`, lê, fecha (drop).

- [ ] **Step 1: Testes que falham** em `main.rs` para a função pura `fn resolve(found: Option<..>, result: io::Result<Option<Vec<u8>>>) -> Status`:
  - `resolve_none_found` → `Error(NoReceiver)`
  - `resolve_permission` (`ErrorKind::PermissionDenied`) → `Error(Permission)`
  - `resolve_timeout` (`Ok(None)`) → `Disconnected`
  - `resolve_absent_reading` (fixture dormindo) → `Disconnected`
  - `resolve_ok` (fixture 82) → `Ok{..}`
- [ ] **Step 2:** `cargo test`. Esperado: FAIL.
- [ ] **Step 3:** Implementar `exchange` (timeout de 300 ms), `resolve` e `main` (imprime `to_json()`, sai com `exit_code()`).
- [ ] **Step 4:** `cargo test`. Esperado: PASS.
- [ ] **Step 5: Hardware.** `cargo build --release && ./target/release/vxe status`. Esperado: JSON com a % igual à do web driver oficial. Com o cabo: `"charging":true`. Mouse dormindo: `{"connected":false}`. Dongle fora: `{"error":"no_receiver"}`, `echo $?` → 1.
- [ ] **Step 6: Tempo.** `for i in 1 2 3 4 5; do /usr/bin/time -f %e ./target/release/vxe status; done`. Esperado: ≤ 0.05 cada.
- [ ] **Step 7:** Commit `feat(cli): vxe status reads battery over hidraw`.

### Task 6: Widget: lógica + ícone C1

**Files:**
- Create: `widget/metadata.json` (Id `com.github.mats.vxe-r1`, KPackageStructure `Plasma/Applet`, `X-Plasma-API-Minimum-Version: "6.0"`, Nome “VXE R1”), `widget/contents/ui/logic.js`, `widget/contents/ui/MouseIcon.qml`, `widget/contents/ui/main.qml`, `widget/tests/tst_logic.qml`

**Interfaces:**
- Consumes: JSON do `vxe status` (Task 3)
- Produces (`logic.js`, `.pragma library`):
  - `parse(stdout) -> {state: "ok"|"disconnected"|"error", battery, charging, mode, error}`. Lixo ou vazio → `{state:"error", error:"cli"}`.
  - `levelColor(battery) -> "#3ccf7a"|"#f0b429"|"#f0525f"`
  - `isCritical(battery) -> bool` (< 15)
  - `label(s) -> "82%"|"––"`
  - `tooltip(s) -> string`: `"VXE R1 — 82%"`, `"VXE R1 — 82% · carregando"`, `"Mouse dormindo ou desligado"`, `"Receptor não encontrado"`, `"Sem permissão — rode install.sh"`, `"Erro ao ler o mouse"`
  - `const REFRESH_MS` = o valor decidido na Task 2
- `MouseIcon.qml`: propriedades `battery`, `charging`, `state`, `textColor`; desenha o C1 aprovado (silhueta com divisão de botões e scroll, preenchimento de baixo para cima proporcional, raio sem contorno quando `charging`, tracejado cinza quando não `ok`). Largura ~2× a altura do painel.

- [ ] **Step 1: Testes que falham** (`TestCase` em `tst_logic.qml`, importando `../contents/ui/logic.js`):
  - `test_parse_ok`: JSON de exemplo → `state "ok"`, `battery 82`, `charging false`, `mode "wireless"`
  - `test_parse_disconnected`, `test_parse_error_permission`
  - `test_parse_garbage`: `""`, `"oops"`, `"{"` → `state "error"`, `error "cli"`
  - `test_level_color`: 100/40 → verde; 39/15 → amarelo; 14/0 → vermelho
  - `test_label`: ok 82 → `"82%"`; disconnected → `"––"`
  - `test_tooltip_permission` e `test_tooltip_charging`: strings exatas de cima
- [ ] **Step 2:** `qmltestrunner -input widget/tests`. Esperado: FAIL.
- [ ] **Step 3:** Implementar `logic.js`.
- [ ] **Step 4:** `qmltestrunner -input widget/tests`. Esperado: PASS.
- [ ] **Step 5:** Implementar `MouseIcon.qml` e `main.qml`: `Plasma5Support.DataSource { engine: "executable" }` rodando `vxe status` no `Component.onCompleted`, num `Timer` de `REFRESH_MS` e ao expandir; `compactRepresentation` = `MouseIcon` + `PlasmaComponents.Label` com `label()`, `font.features: {"tnum": 1}`, semibold; `toolTipMainText` = `tooltip()`. Achar o binário em `~/.local/bin/vxe` (caminho absoluto via `StandardPaths`/`Qt.resolvedUrl`-free string montada a partir de `HOME`), já que o PATH do plasmashell pode não incluir `~/.local/bin`.
- [ ] **Step 6: Visual.** `kpackagetool6 -t Plasma/Applet -i widget` (ou `-u` se já instalado), adicionar ao painel ao lado da bandeja. Comparar com `polish-v1.html`/`charging-v3.html` em tema escuro e claro: normal, < 40%, < 15%, com cabo (raio), mouse dormindo. Tirar screenshot de cada um para o usuário aprovar.
- [ ] **Step 7:** Commit `feat(widget): tray icon with battery level`.

### Task 7: Widget: popup B

**Files:**
- Create: `widget/contents/ui/Popup.qml`
- Modify: `widget/contents/ui/main.qml` (`fullRepresentation: Popup {}`), `widget/contents/ui/logic.js`, `widget/tests/tst_logic.qml`

**Interfaces:**
- Consumes: o objeto de `parse()` e o timestamp da última leitura
- Produces: `logic.js`:
  - `connectionText(s) -> "2.4 GHz · conectado"|"cabo · carregando"|"cabo"|"desconectado"`
  - `stateText(s) -> "carregando"|"descarregando"|""`
  - `updatedText(msAgo) -> "atualizado agora"|"atualizado há 1 min"|"atualizado há N min"`

- [ ] **Step 1: Testes que falham:** `test_connection_text` (wireless ok; wired + charging; wired sem carregar; disconnected), `test_state_text`, `test_updated_text` (0 → “agora”, 59 999 → “agora”, 60 000 → “há 1 min”, 180 000 → “há 3 min”).
- [ ] **Step 2:** `qmltestrunner -input widget/tests`. Esperado: FAIL.
- [ ] **Step 3:** Implementar as funções.
- [ ] **Step 4:** `qmltestrunner -input widget/tests`. Esperado: PASS.
- [ ] **Step 5:** Implementar `Popup.qml` conforme o mockup B: cabeçalho (ícone do mouse em traço, “VXE R1”, `connectionText` dimmed, % grande à direita em `Kirigami.Theme`), barra fina de 6 px na `levelColor`, linha com `stateText` à esquerda e `updatedText` à direita. Sem seção DPI. Cores do tema via `Kirigami.Theme`. Abrir o popup dispara uma nova leitura.
- [ ] **Step 6: Visual.** Reinstalar o widget, abrir o popup nos temas escuro e claro e comparar com `polish-v1.html`. Screenshot para o usuário.
- [ ] **Step 7:** Commit `feat(widget): battery popup`.

### Task 8: `install.sh` e verificação ponta a ponta

**Files:**
- Create: `install.sh`

- [ ] **Step 1:** `install.sh` (bash, `set -euo pipefail`): `cargo build --release --manifest-path cli/Cargo.toml`; copia para `~/.local/bin/vxe`; `kpackagetool6 -t Plasma/Applet -i widget || kpackagetool6 -t Plasma/Applet -u widget`; se `/etc/udev/rules.d/70-vxe-r1.rules` for diferente de `udev/70-vxe-r1.rules`, roda `sudo install -m644 ...` + `udevadm control --reload` + `udevadm trigger` (único passo com sudo, com aviso antes).
- [ ] **Step 2:** `shellcheck install.sh` (se disponível). Esperado: sem avisos.
- [ ] **Step 3: Ponta a ponta** (usuário presente): rodar `./install.sh`, reiniciar o plasmashell (`systemctl --user restart plasma-plasmashell`), confirmar: (a) % igual à do web driver; (b) cabo → raio aparece e % sobe; (c) dongle fora → tracejado + tooltip “Receptor não encontrado”; (d) `ps aux | grep vxe` sem processo residente entre leituras; (e) mouse parado entra em repouso normalmente com o widget ativo.
- [ ] **Step 4:** Commit `feat: install script`.

---

**Depois deste plano:** com `docs/protocol.md` em mãos, escrever o plano da Fase 2 (DPI: `vxe dpi <valor>` + seção do popup), seguindo a ordem de mecanismos do spec e sem gravar config sem perguntar.
