# VXE R1 na bandeja — Fase 2 (DPI) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Trocar o DPI do mouse pelo popup, regravando o valor do estágio ativo, e mostrar o DPI atual.

**Architecture:** A CLI ganha leitura/escrita de EEPROM (cmds `0x08`/`0x07`), a tabela de DPI do sensor, `vxe dpi <valor>` e o campo `dpi` em `vxe status`. O widget ganha uma seção DPI com botões de valores fixos que chama `vxe dpi N`.

**Tech Stack:** igual à fase 1 (Rust só `std`, QML Plasma 6, `qmltestrunner`).

**Spec:** `docs/superpowers/specs/2026-11-17-vxe-r1-tray-design.md` (seção “Escopo e fases”, decisão de 2026-11-17) e `docs/protocol.md` (seção DPI).

## Global Constraints

- Nunca gravar com o mouse dormindo: `0x03` primeiro; se offline → `{"connected":false}`, nada gravado, saída 0.
- Gravar **só** o registro de 4 bytes do estágio ativo (`0x000c + 4·ativo`). Nada mais na EEPROM.
- Antes da primeira gravação: salvar `0x0000..0x002b` (estágios e cabeçalho) em `$XDG_STATE_HOME/vxe/backup.hex` (padrão `~/.local/state/vxe/backup.hex`), **só se o arquivo não existir**.
- Toda gravação: esperar o eco (timeout 500 ms), depois ler o registro de volta e comparar. Divergência → `{"error":"write_failed"}`, saída 1.
- DPI aceito: 50–10 000, múltiplo de 50 (codificação de tabela, `ex = 0`). Fora disso → uso no stderr, saída 2, nada enviado.
- Valores do seletor no popup: **400 / 800 / 1200 / 1600 / 3200** (chamada do Claude). O ativo é destacado quando `dpi` bate com um deles.
- Botão do mouse continua trocando de estágio; o app nunca muda o índice do estágio ativo.

## Review Focus

1. **Mouse dorme entre o `0x03` e a gravação.** Esperado: timeout no eco → `write_failed`, sem gravação parcial assumida como ok. Teste: Task 2 `dpi_echo_timeout_is_write_failed`.
2. **Registro do estágio ativo com checksum inválido na leitura** (EEPROM estranha). Esperado: não gravar, `{"error":"io"}`. Teste: Task 1 `decode_stage_rejects_bad_check`.
3. **Índice de estágio ativo fora de 0–7 ou par inválido.** Esperado: `io`, nada gravado. Teste: Task 2 `dpi_bad_active_stage_is_io`.
4. **Clique duplo rápido no popup** (dois `vxe dpi` simultâneos). Esperado: o widget ignora cliques enquanto um `vxe dpi` está em andamento. Teste: Task 3 `test_dpi_busy_blocks`.
5. **Mouse no cabo.** Mesmo caminho de gravação; deve funcionar igual. Verificação de hardware na Task 2.

---

### Task 1: Protocolo de EEPROM e tabela de DPI (puro)

**Files:** Modify `cli/src/protocol.rs`. Fixtures: `cli/tests/fixtures/eeprom_0000.hex`, `eeprom_000c.hex` (já existem).

**Interfaces (Produces):**
- `pub fn read_request(addr: u16, len: u8) -> Vec<u8>` (cmd `0x08`, `len` 1–10)
- `pub fn write_request(addr: u16, data: &[u8]) -> Vec<u8>` (cmd `0x07`, 1–10 bytes)
- `pub fn parse_read(addr: u16, len: u8, resp: &[u8]) -> Result<Vec<u8>, ParseError>`: valida frame, eco de endereço e `len`.
- `pub fn is_write_echo(req: &[u8], resp: &[u8]) -> bool`: resposta igual ao request nos 17 bytes.
- `pub fn dpi_to_code(dpi: u16) -> Option<u8>` / `pub fn code_to_dpi(code: u8) -> Option<u16>` (tabela da `docs/protocol.md`)
- `pub fn decode_pair(v: u8, c: u8) -> Option<u8>` (`v + c == 0x55`)
- `pub fn encode_stage(dpi: u16) -> Option<[u8; 4]>` / `pub fn decode_stage(rec: &[u8]) -> Option<u16>` (x == y, `ex == 0`, checksum)
- `pub const STAGE_BASE: u16 = 0x000c; pub const ACTIVE_STAGE_ADDR: u16 = 0x0004;`
- `is_reply_to` passa a casar também por endereço quando `cmd` é `0x08`/`0x07` (usar `parse_read`/`is_write_echo` no transporte).

- [ ] **Step 1: Testes que falham:** `read_request_bytes` (`read_request(0x000c, 10)` = `08 08 00 00 0c 0a 00…00 <ck>` com 17 bytes que somam `0x55`); `write_request_bytes` (soma `0x55`, dados em `[6..]`); `parse_read_fixture_000c` → `12 12 00 31 1c 1c 00 1d 25 25`; `parse_read_rejects_other_addr`; `dpi_table_known_codes` (800↔`0x12`, 1200↔`0x1c`, 1600↔`0x25`, 3200↔`0x4b`, 400↔`0x09`, 50↔1, 10000↔235); `dpi_table_rejects` (`dpi_to_code(825)`, `(0)`, `(10050)` = None; `code_to_dpi(7)`, `(0)`, `(236)` = None); `encode_stage_1200` = `[0x1c,0x1c,0x00,0x1d]`; `decode_stage_fixture` (os 3 registros do fixture → 800/1200/1600); `decode_stage_rejects_bad_check`; `decode_pair_active_stage` (fixture `0x0000`, bytes do endereço 4 → `Some(1)`).
- [ ] **Step 2:** `cargo test` → FAIL.
- [ ] **Step 3:** Implementar. Tabela gerada por função (iterar códigos 1..=235 pulando a lista), não literal de 200 entradas.
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5:** Commit `feat(cli): EEPROM frames and DPI table`.

### Task 2: `vxe dpi <valor>` e `dpi` no status (hardware)

**Files:** Modify `cli/src/main.rs`, `cli/src/status.rs`, `cli/src/hidraw.rs` (exchange recebe um matcher `Fn(&[u8]) -> bool` em vez de usar só o cmd).

**Interfaces:**
- `Status::Ok` ganha `dpi: Option<u16>`; JSON acrescenta `,"dpi":1200` quando `Some` (ordem: `connected, mode, battery, charging, dpi`). `ErrKind::WriteFailed` → `{"error":"write_failed"}`.
- `fn read_dpi(ex: &mut Exchange) -> Result<Option<u16>, ErrKind>`: lê `0x0004` (2 bytes) e o registro ativo.
- `fn set_dpi(mode, dpi, ex, backup: &mut dyn FnMut(&[u8]) -> io::Result<()>) -> Status`: `0x03` → backup (lendo `0x0000..0x002b` em blocos de 10) → grava registro → eco → lê de volta → compara → devolve `Status::Ok` completo.
- `vxe status` inclui `dpi` (falha ao ler DPI não derruba a bateria: `dpi: None`).

- [ ] **Step 1: Testes que falham** (exchange falso como na fase 1, agora casando por cmd+endereço): `json_ok_with_dpi`; `json_write_failed`; `status_includes_dpi`; `dpi_sleeping_writes_nothing` (só `0x03` enviado); `dpi_writes_only_active_record` (único `0x07` enviado é em `0x0010` com `1c 1c 00 1d` quando ativo=1, dpi=1200); `dpi_backup_once` (backup chamado com 44 bytes antes da escrita); `dpi_echo_timeout_is_write_failed`; `dpi_readback_mismatch_is_write_failed`; `dpi_bad_active_stage_is_io`; `dpi_arg_validation` (`825`, `0`, `abc` → saída 2 sem I/O).
- [ ] **Step 2:** `cargo test` → FAIL.
- [ ] **Step 3:** Implementar. Backup: escrever o arquivo só se não existir (`create_new`); falha de backup → `io`, nada gravado.
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Hardware (usuário presente):** `vxe status` → `"dpi":1200`. `vxe dpi 1000` → `"dpi":1000`; o usuário sente a diferença; `cat ~/.local/state/vxe/backup.hex` existe. Apertar o botão de DPI do mouse uma volta inteira → voltar ao estágio 2 → `vxe status` = 1000. `vxe dpi 1200` → volta ao original. Repetir `vxe dpi 1000` no cabo.
- [ ] **Step 6: Persistência:** com 1000 gravado, desligar e ligar o mouse (chave embaixo) → `vxe status`. Se voltar 1200, **parar e perguntar ao usuário** antes de adicionar o “commit por troca de perfil” do atk-control. Restaurar 1200 no fim.
- [ ] **Step 7:** Commit `feat(cli): vxe dpi rewrites the active stage`.

### Task 3: Seção DPI no popup

**Files:** Modify `widget/contents/ui/logic.js`, `widget/contents/ui/Popup.qml`, `widget/contents/ui/main.qml`, `widget/tests/tst_logic.qml`.

**Interfaces:**
- `parse()` passa a levar `dpi` (número ou `undefined`); `merge()` mantém `dpi` no estado `asleep`.
- `var DPI_PRESETS = [400, 800, 1200, 1600, 3200]`
- `function dpiCommand(dpi) -> '"$HOME/.local/bin/vxe" dpi 1200'` (só inteiros de `DPI_PRESETS`; senão `""`)
- `main.qml`: `property bool dpiBusy`; `function setDpi(n)` ignora se `dpiBusy`; usa a mesma `DataSource`, distinguindo a fonte pelo comando; a resposta atualiza `view` como no status.
- `Popup.qml`: seção “DPI” (rótulo pequeno em caixa alta) + seletor segmentado do mockup B com os 5 valores; ativo na cor de acento; desabilitado (opaco) quando `!hasLevel` ou `dpiBusy`. Erro `write_failed` → tooltip/subtítulo “Não foi possível mudar o DPI”.

- [ ] **Step 1: Testes que falham:** `test_parse_dpi`; `test_merge_asleep_keeps_dpi`; `test_dpi_command` (1200 → string exata; 1000 e `"1;rm"` → `""`); `test_dpi_busy_blocks` (função pura `canSetDpi(view, busy)` → false quando busy ou sem nível); `test_tooltip_write_failed`.
- [ ] **Step 2:** `qmltestrunner` → FAIL.
- [ ] **Step 3:** Implementar logic.js.
- [ ] **Step 4:** `qmltestrunner` → PASS.
- [ ] **Step 5:** Implementar Popup/main; renderizar offscreen (como na fase 1) e conferir com o mockup B.
- [ ] **Step 6: Ponta a ponta (usuário):** `./install.sh`, reiniciar plasmashell, abrir popup → 1200 destacado; clicar 800 → mouse muda, 800 destacado; voltar a 1200.
- [ ] **Step 7:** Commit `feat(widget): DPI selector in popup`.
