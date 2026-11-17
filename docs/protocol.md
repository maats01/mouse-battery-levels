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
| `0x03` receptor online | `08 03 00…00 4a` | `[5]` = `1` se o mouse está ligado ao receptor |
| `0x04` bateria | `08 04 00…00 49` | `[5]` = len, `[6]` = %, `[7]` = carregando (0/1), `[8..9]` = mV big-endian (fora do `len`; informativo) |

Exemplos reais (fixtures em `cli/tests/fixtures/`):

- sem fio, 30%, descarregando: `08 04 00 00 00 02 1e 00 0e 04 00 00 00 00 00 00 17`
- cabo, 50%, carregando: `08 04 00 00 00 02 32 01 0e 98 00 00 00 00 00 00 6e`
- receptor online: `08 03 00 00 00 01 01 00 00 00 00 00 00 00 00 00 48`

Tempo de resposta: ~13 ms sem fio, ~4 ms no cabo.

Mouse ausente / desligado: sem resposta ao `0x04` dentro do timeout, ou `0x03` com
`[5] = 0`. A CLI faz `0x03` antes do `0x04` no caminho sem fio e não pergunta a bateria
se o mouse estiver offline.

## Repouso

Registro de repouso (`0x00a9`, par 3): `3` × 10 s = 30 s.
Resultado do teste de “a consulta acorda o mouse?”: ver seção abaixo (preenchida após o
teste sem tocar no mouse).

## DPI (fase 2 — só leitura feita)

- `0x0000` (pares valor/check): polling `01` = 1000 Hz, 4 estágios, estágio ativo `1`
  (base 0).
- `0x000c`: registros de 4 bytes `x y modo check`, codificação step50
  (`(byte + 1) × 50`): 950 / 1450 / 1900 / 3800 DPI.
- Escrita (`0x07`) no endereço do estágio ativo é o caminho provável para “trocar de
  estágio”; **não testado**. O atk-control também documenta um commit por troca de
  perfil para persistir. Decidir na fase 2, perguntando ao usuário antes de gravar.
