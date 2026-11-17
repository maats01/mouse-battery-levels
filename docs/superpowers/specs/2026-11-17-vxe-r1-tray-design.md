# VXE R1 — bateria e DPI na bandeja do Plasma

Data: 2026-11-17

## Intenção

Ver o nível de bateria do mouse VXE R1 de relance, na barra do KDE Plasma, e trocar o estágio de DPI com um clique — num app **simples, performático e muito leve**, limpo e minimalista. O usuário não quer um painel de configuração completo (como o atk-control), só bateria + troca de DPI.

Por que precisa existir: o kernel não expõe a bateria desse mouse. O driver `hid-kysona` (presente no kernel do usuário) só reconhece receptores `3554:*` e `373b:1085`; o receptor do usuário é **`373b:10c9` ("Compx NK mouse NANO dongle")**, então nada aparece em `/sys/class/power_supply` nem no UPower.

## Ambiente-alvo (verificado na máquina)

- CachyOS, kernel 7.3, **KDE Plasma 6.8.2 em Wayland**, Qt 6.12
- `org.kde.plasma.plasma5support` disponível (data engine `executable`)
- Rust/cargo 1.99 instalados
- Receptor `373b:10c9` com 3 interfaces: `hidraw0`, `hidraw1`, `hidraw2` (hoje `root:root 0600`)
- O usuário **carrega o mouse pelo cabo e continua usando**. Com cabo, o mouse pode aparecer como outro dispositivo USB (IDs com fio citados pela comunidade para o R1: `3554:f58f`, `373b:1271`) — o ID real do mouse do usuário é descoberto no spike com `lsusb` com o cabo plugado.

## Objetivos

- Ícone ao lado da bandeja: silhueta de mouse com preenchimento de nível + porcentagem ao lado.
- Clique abre popup com bateria e seletor de estágio de DPI; clicar num estágio aplica no mouse.
- Zero processo residente além do próprio Plasma; leitura rápida e rara.
- Funciona sem sudo.
- **Não acordar o mouse desnecessariamente** (pedido explícito do usuário).

## Escopo e fases (decidido pelo usuário)

- **Fase 1 — bateria:** CLI `vxe status` + widget com ícone e popup (sem a seção de DPI, ou com ela oculta). Entregue assim que a leitura de bateria funcionar, mesmo que o DPI ainda não.
- **Fase 2 — DPI:** troca de DPI no popup, quando o protocolo para isso estiver confirmado.
- Uso pessoal, uma máquina; sem empacotamento para distribuição (chamada do Claude).

## Não-objetivos

- Editar os valores dos estágios de DPI (opção “B”, possível no futuro sem refazer nada).
- Polling rate, botões, RGB, macros, perfis, firmware.
- Outros desktops além do KDE Plasma 6.
- Outros mouses (estruturar o código para não impedir, mas não suportar agora).
- Notificações de bateria baixa (não pedido).

## Anti-objetivos (falha mesmo que “funcione”)

- Daemon rodando em segundo plano consumindo memória/CPU.
- Visual pesado/“bold” — o usuário rejeitou explicitamente traços grossos e contornos extras.
- Precisar de sudo/root para uso normal.
- Brigar com o software do mouse: abrir o hidraw e segurá-lo aberto.

## Abordagens consideradas

1. **Widget do Plasma (QML) + utilitário CLI em Rust — ESCOLHIDA.** A CLI fala HID com o receptor e sai; o widget chama a CLI periodicamente e ao clicar. Única opção que entrega o ícone largo e o popup ancorado no ícone no Wayland, e a mais leve (nada residente). Custo: só KDE.
2. App de bandeja Rust independente (StatusNotifierItem). Portável, mas o Plasma força ícone quadrado (C1 ficaria espremido) e no Wayland o app não consegue posicionar um popup ao lado do ícone — perderia as duas escolhas visuais.
3. App Qt/C++ com QSystemTrayIcon. Mesmos limites da 2, mais pesado.

## Design

### Componente 1 — CLI `vxe` (Rust)

Binário único, sem runtime, dependências mínimas.

- `vxe status` → imprime **uma linha JSON** e sai:
  `{"connected":true,"mode":"wireless","battery":82,"charging":false,"dpi":800,"dpi_stage":1,"dpi_stages":[400,800,1600,3200]}`
  - `mode`: `"wireless"` (pelo receptor) ou `"wired"` (pelo cabo). Com cabo plugado, a CLI deve preferir o dispositivo com fio se for ele que reporta bateria/carregando; o spike define qual caminho dá o dado confiável.
  - Mouse dormindo/desligado (receptor presente, mouse não responde): `{"connected":false}`.
  - Receptor ausente: `{"error":"no_receiver"}`; sem permissão: `{"error":"permission"}`. Código de saída ≠ 0 nesses casos.
- `vxe dpi <valor>` (fase 2) → aplica o DPI, imprime o mesmo JSON de `status` já atualizado. Ordem de preferência do mecanismo, conforme o que o protocolo oferecer:
  1. comando para ativar um estágio existente;
  2. comando que define o DPI atual diretamente (o usuário aceitou explicitamente trocar o “DPI geral” em vez de estágio, se for mais fácil);
  3. ler–alterar–gravar o bloco de configuração: **não fazer sem confirmar com o usuário antes**, porque pode sobrescrever ajustes do software oficial. Se for o único caminho, propor com backup do bloco e verificação antes de gravar.
- Localiza o dispositivo varrendo `/sys/class/hidraw/*/device/uevent` pelo receptor (`HID_ID=0003:0000373B:000010C9`) e pelo ID com fio descoberto no spike, escolhe a interface de configuração (a que tem o report de vendor), abre, faz a troca, **fecha**. Timeout curto (~300 ms) por requisição.
- Deve terminar em poucos ms no caso normal.

### Protocolo (a descobrir — primeiro passo do trabalho)

O comando exato para o `373b:10c9` **não está confirmado**. Referências da comunidade para receptores Compx/ATK `373b`:
- [MouseBattery](https://github.com/zhijuyingfeng/MouseBattery) (C++/Qt, hidraw, bateria + carregando): indica Report ID `0x08`, reports de 16 bytes (“Compx/ATK v1”).
- [atk-control](https://github.com/NasirA1991/atk-control) (bateria na bandeja, DPI, suporte VXE R1 com receptores 4K/8K).
- [OpenVXE](https://github.com/FrancescoLFM/OpenVXE), [OpenMouse PR #44](https://github.com/OpenMouse-Project/mouse-protocol/pull/44), driver `hid-kysona` do kernel (fonte em `drivers/hid/hid-kysona.c`).
- O web driver oficial da VXE usa WebHID; capturar o tráfego dele (ex.: com `usbmon`/Wireshark) é o plano B.

Primeiro entregável: um **spike** no hardware real que responde:
- como ler bateria (e carregando) pelo receptor, e também com o cabo plugado;
- **se a consulta de bateria acorda o mouse** ou é respondida pelo receptor a partir de valor em cache (ex.: deixar o mouse dormir, consultar, ver se ele acorda / se a resposta é “desconectado”);
- como ler e trocar o DPI (fase 2; não bloqueia a fase 1). Só depois disso a CLI é finalizada. Se o receptor não reportar “carregando”, o raio simplesmente nunca aparece; se não reportar a lista de estágios, ver “Decisões em aberto”.

### Componente 2 — Widget do Plasma 6 (QML)

Pacote plasmoid (`metadata.json`, `contents/ui/main.qml`), adicionado ao painel ao lado da bandeja (não dentro dela — dentro da bandeja os ícones são quadrados).

- Chama `vxe status` via data engine `executable` do `plasma5support` ao abrir o popup e periodicamente. **Intervalo (chamada do Claude, a partir do spike):** 60 s se a consulta não acorda o mouse; se acordar, 5 min, e a consulta nunca deve impedir o mouse de dormir. Bateria muda devagar — precisão de minutos é suficiente.
- Clique num estágio → `vxe dpi N`, atualiza a UI com a resposta.
- Usa cores do tema (Kirigami/PlasmaCore) para contorno e texto, para funcionar em tema claro e escuro.

**Ícone (compactRepresentation) — design “C1 refinado” aprovado:**
- ~44×22 px: silhueta de mouse (corpo arredondado, divisão dos botões e scroll, traço ~1.5 px na cor do texto do tema) à esquerda, porcentagem à direita (peso semibold, **algarismos tabulares** para não “pular” de largura).
- O mouse é preenchido de baixo para cima como líquido, proporcional ao nível. Cor: verde (≥40%), amarelo (15–39%), vermelho (<15%). Abaixo de 15% o contorno e o número também ficam vermelhos.
- **Carregando:** mesmas cores por nível (sem cor especial); apenas um **raio pequeno, fino, branco, sem contorno** no centro do corpo do mouse.
- **Mouse dormindo/desconectado:** contorno tracejado cinza, texto “––”.
- **Erro (sem receptor/permissão):** como desconectado; tooltip explica.
- Visual clean, traços finos — nada de bold extra.

**Popup (fullRepresentation) — design “B” aprovado:**
```
┌──────────────────────────────┐
│ 🖱  VXE R1              82%  │
│     2.4 GHz · conectado      │
│ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░        │
│ descarregando  atualizado 1m │
│ DPI                          │
│ [ 400 |▌800▐| 1600 | 3200 ]  │
└──────────────────────────────┘
```
- Cabeçalho: ícone, nome, estado da conexão (“2.4 GHz · conectado” ou “cabo · carregando”), % grande à direita.
- Barra de bateria fina com a cor do nível; linha de estado (carregando/descarregando) e “atualizado há X”.
- Seletor segmentado com os valores de DPI (os estágios do mouse, se legíveis; senão uma lista fixa padrão — chamada do builder); o ativo destacado na cor de acento do tema. Clicar aplica o valor. Oculto na fase 1.

Mockups de referência: `.superpowers/brainstorm/*/content/polish-v1.html` (popup e ícone) e `charging-v3.html` (raio final).

### Componente 3 — Permissões

Regra udev `/etc/udev/rules.d/70-vxe-r1.rules`:
`SUBSYSTEM=="hidraw", ATTRS{idVendor}=="373b", ATTRS{idProduct}=="10c9", TAG+="uaccess"`
mais uma linha igual para o ID do modo com fio descoberto no spike. Dá acesso ao usuário logado sem sudo.

### Instalação

- `cargo build --release` → `~/.local/bin/vxe`
- `kpackagetool6 -t Plasma/Applet -i widget/` → widget instalado no usuário
- Regra udev instalada uma vez com sudo (único passo privilegiado), seguida de `udevadm control --reload && udevadm trigger`.
- Um `install.sh` que faz os três.

## Decisões em aberto / deixadas para o builder

- Formato exato dos pacotes HID (definido pelo spike).
- Escolha de crate HID: ler/escrever `/dev/hidrawN` direto (preferido, sem dependências) vs `hidapi`.
- Estrutura interna do QML e do Rust.
- Se o mouse não informar os valores dos estágios, o widget usa uma lista fixa de valores comuns (chamada do builder, sem bloquear).
- Escuta passiva de reports que o mouse envia sozinho foi descartada: exigiria processo residente segurando o hidraw (anti-objetivo).

## Sucesso

- Ícone mostra a % correta (bate com o web driver oficial) e muda de cor nos limites.
- Com o cabo plugado, o ícone mostra o raio e a % sobe.
- Deixar o widget rodando não impede o mouse de entrar em repouso.
- (Fase 2) Clicar num valor muda o DPI do mouse de verdade, e a mudança reflete no widget.
- `vxe status` roda em < 50 ms; nenhum processo extra fica residente.
- Funciona sem sudo após a instalação.
