# Protocolo VXE R1 S NK — o que o app usa

Levantado no spike (Task 2) no hardware real em 2026-11-17, com base nas notas do
[atk-control](https://github.com/NasirA1991/atk-control) (`docs/PROTOCOL.md`, “COMPX v1”).

## Dispositivos

| Caminho | USB ID | Nome | Quando aparece |
|---|---|---|---|
| sem fio | `373b:10c9` | Compx NK mouse NANO dongle | receptor plugado |
| cabo | `373b:121f` | Compx VXE R1 S NK | cabo ligado no mouse (carregando) |

O usuário usa um dock que segura o receptor e o cabo: para carregar, o cabo sai do
receptor e vai para o mouse. **Os dois nunca aparecem juntos.** Se aparecerem, a CLI
prefere o cabo (é quem sabe que está carregando).

Identidade (cmd `0x10`): CID/MID `2,0x75` nos dois caminhos.

## Interface

Cada dispositivo expõe 3 hidraw. A de configuração é a única cujo report descriptor
contém a coleção vendor `06 02 ff 09 02 a1 01 85 08` (usage page `0xFF02`, usage `2`,
report ID `0x08`, 16 bytes). No sistema do usuário foi `hidraw1` nos dois casos, mas a
CLI identifica pelo descriptor, nunca pelo número.

## Frame

Escrita e leitura: 17 bytes = report ID `0x08` + 16 bytes de payload.

```
08 <cmd> <status> <addr_hi> <addr_lo> <len> <data…> <checksum>
```

`checksum` = `(0x55 - ((0x08 + soma(payload[0..15])) & 0xff)) & 0xff`, ou seja, os 17
bytes somam `0x55` mod 256. Resposta válida: `[0]=0x08`, `[1]=cmd`, `[2]=0` (status),
checksum confere.

## Comandos usados

| cmd | request (17 bytes) | resposta |
|---|---|---|
| `0x03` receptor online | `08 03 00…00 4a` | `[5]` = len (1), `[6]` = `1` mouse acordado, `0` dormindo/desligado. Respondido pelo próprio receptor (~3 ms) |
| `0x04` bateria | `08 04 00…00 49` | `[5]` = len, `[6]` = %, `[7]` = carregando (0/1), `[8..9]` = mV big-endian (fora do `len`; informativo) |

Exemplos reais (fixtures em `cli/tests/fixtures/`):

- sem fio, 30%, descarregando: `08 04 00 00 00 02 1e 00 0e 04 00 00 00 00 00 00 17`
- cabo, 50%, carregando: `08 04 00 00 00 02 32 01 0e 98 00 00 00 00 00 00 6e`
- receptor online: `08 03 00 00 00 01 01 00 00 00 00 00 00 00 00 00 48`
- receptor, mouse dormindo: `08 03 00 00 00 01 00 00 00 00 00 00 00 00 00 00 49`

Tempo de resposta: ~13 ms sem fio, ~4 ms no cabo.

Mouse ausente / desligado: sem resposta ao `0x04` dentro do timeout, ou `0x03` com
`[6] = 0`. A CLI faz `0x03` antes do `0x04` no caminho sem fio e não pergunta a bateria
se o mouse estiver offline.

## Repouso

Registro de repouso (`0x00a9`, par 3): `3` × 10 s = 30 s.
Teste (mouse parado 75 s, sem nenhuma consulta no meio): `0x03` respondeu `[6] = 0`
em 3 ms (o receptor sabe que o mouse dorme) e `0x04` deu timeout duas vezes. Enquanto o
mouse está em uso, `0x03` responde `1`.

Conclusão: consultar `0x03` não envolve o mouse; a CLI só manda `0x04` quando `0x03`
diz que ele está acordado, então **nunca acorda o mouse**. Intervalo do widget:
`REFRESH_MS = 60000`.

## DPI (fase 2)

Não existe comando de “DPI atual” volátil: o DPI só existe nos registros de estágio da
EEPROM. Leitura `0x08` / escrita `0x07` (frame `08 <cmd> 00 <addr_hi> <addr_lo> <len>
<data…> <ck>`; resposta de leitura traz os dados a partir de `[6]`; a escrita é confirmada
pelo eco do próprio request em ~80 ms).

- `0x0000` (pares valor/`0x55-valor`): polling `01` = 1000 Hz; `0x0002` = 4 estágios;
  `0x0004` = estágio ativo `1` (base 0).
- `0x000c + 4·n`: estágio n como `[x, y, ex, 0x55-(x+y+ex)]`.
- Codificação (PAW3395SE, conforme OpenVXE, verificada em hardware): `x`/`y` são índice
  numa tabela — DPI de 50 a 10 000 em passos de 50 mapeiam para os códigos 1–235, pulando
  7, 13, 20, 26, 33, 40, 46, 53, 60, 66, 73, 80, 86, 93, 100, 106, 113, 120, 126, 133,
  140, 146, 153, 160, 166, 173, 180, 186, 193, 200, 206, 213, 220, 226, 233. `ex = 0`
  até 10 000.
- Lido no mouse do usuário: `12 12 00 31`, `1c 1c 00 1d`, `25 25 00 0b`, `4b 4b 00 bf` =
  **800 / 1200 / 1600 / 3200**, estágio ativo 1200 — **confirmado pelo usuário**. (A
  leitura “step50” 950/1450/1900/3800 anotada antes estava errada.)

Fixtures de leitura:
- `0x0000`: `08 08 00 00 00 0a 01 54 04 51 01 54 00 55 00 55 92`
- `0x000c`: `08 08 00 00 0c 0a 12 12 00 31 1c 1c 00 1d 25 25 3b`
