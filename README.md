# mouse-battery-levels

Widget minimalista para o **KDE Plasma 6** que mostra a bateria do mouse **VXE R1 S NK** na barra e permite trocar o DPI. Por trás dele há um utilitário de linha de comando bem pequeno, `vxe`, em Rust e sem dependências.

- **Ícone na barra:** um mouse que vai sendo "preenchido" conforme o nível, com a porcentagem ao lado. Fica verde a partir de 40%, amarelo de 15% a 39% e vermelho abaixo de 15%. Enquanto carrega, aparece um raio pequeno.
- **Popup ao clicar:** mostra a bateria, a conexão (2.4 GHz ou cabo) e um slider de DPI. Clicando no número do DPI, dá para digitar um valor.
- **Leve:** nenhum processo fica rodando em segundo plano. O widget chama `vxe status` uma vez por minuto, e cada leitura leva uns 20 ms.
- **Não acorda o mouse:** primeiro o app pergunta ao receptor se o mouse está acordado, e só pede a bateria nesse caso. Enquanto o mouse dorme, o widget mostra a última leitura.

## Por que existe

O kernel não expõe a bateria desse mouse em `/sys/class/power_supply`, então o UPower e o indicador de bateria do Plasma não a mostram. O driver `hid-kysona` só reconhece outros receptores. Este projeto fala direto com o receptor pelo protocolo HID de configuração da Compx/ATK, que está documentado em [`docs/protocol.md`](docs/protocol.md).

## Requisitos

- KDE Plasma 6 (testado no 6.8, Wayland)
- Rust (`cargo`) para compilar a CLI
- `kpackagetool6`, que já vem com o Plasma
- `sudo` uma única vez, para instalar a regra udev

## Instalação

```bash
git clone https://github.com/maats01/mouse-battery-levels.git
cd mouse-battery-levels
./install.sh
```

O `install.sh`:

1. compila `vxe` e o copia para `~/.local/bin/vxe`;
2. instala ou atualiza o widget **"VXE R1"** no seu usuário;
3. instala `/etc/udev/rules.d/70-vxe-r1.rules`, que dá ao usuário logado acesso ao mouse sem `sudo`. Só pede senha quando a regra está ausente ou desatualizada.

Depois disso, clique com o botão direito no painel, escolha **Adicionar widgets…**, procure **VXE R1** e arraste o widget para perto da bandeja. Ele precisa ficar no painel, e não dentro da bandeja, para o ícone ter largura suficiente para o número.

Se você atualizar o widget e não vir mudança, reinicie o painel:

```bash
systemctl --user restart plasma-plasmashell
```

## Uso

### No widget

- **Bateria:** fica no ícone e no popup. Se nunca houve leitura ou o receptor não está plugado, aparece `––`.
- **DPI pelo slider:** arraste entre 400 e 3200, em passos de 50. O valor só é gravado quando você solta.
- **DPI digitado:** clique no número e digite qualquer valor entre 50 e 10 000. **Enter** aplica e **Esc** cancela. Valores que não são múltiplos de 50 são arredondados.
- Os pontinhos no slider marcam os outros estágios de DPI configurados no mouse.

### Na linha de comando

```bash
vxe status
# {"connected":true,"mode":"wireless","battery":45,"charging":false,"dpi":1200,"stages":[800,1200,1600,3200]}

vxe dpi 1600
# grava 1600 no estágio ativo e imprime o status atualizado
```

| Saída | Significado |
|---|---|
| `{"connected":false}` | mouse dormindo ou desligado; nada é enviado a ele |
| `{"error":"no_receiver"}` | receptor e cabo ausentes |
| `{"error":"permission"}` | sem acesso ao hidraw; rode `./install.sh` |
| `{"error":"write_failed"}` | o mouse não confirmou a gravação do DPI |
| `{"error":"io"}` | resposta inválida, ou memória do mouse com conteúdo inesperado |

Os códigos de saída são `0` para sucesso ou mouse dormindo, `1` para erro e `2` para argumento inválido.

## Como funciona a troca de DPI

O protocolo **não tem um comando de "DPI atual"**: o DPI só existe nos estágios gravados na memória do mouse, os mesmos que o botão embaixo dele alterna. Por isso, `vxe dpi N` **regrava o valor do estágio ativo**:

- se o estágio 2 era 1200 e você escolhe 1000, o estágio 2 passa a ser 1000;
- o botão de DPI continua alternando entre os estágios normalmente;
- o valor sobrevive a desligar e ligar o mouse.

Por segurança:

- **Backup:** antes da primeira gravação, os estágios originais são salvos em `~/.local/state/vxe/backup.hex`. Esse arquivo nunca é sobrescrito.
- **Validação:** a CLI só grava se o cabeçalho e todos os registros de estágio estiverem íntegros. Ela grava apenas os 4 bytes do estágio ativo.
- **Confirmação:** toda gravação precisa ser confirmada pelo mouse e é lida de volta. Se não bater, o resultado é `write_failed`.
- **Mouse dormindo:** com o mouse dormindo, nada é gravado.

## Limitações

- **Só testado com um mouse:** o VXE R1 S NK (CID/MID `2,117`, sensor PAW3395SE), pelo receptor NearLink `373b:10c9` e pelo cabo (`373b:121f`). Outros receptores e mouses da família Compx/ATK provavelmente usam o mesmo protocolo, mas não são reconhecidos. Os IDs ficam em `cli/src/device.rs` e `udev/70-vxe-r1.rules`.
- **Codificação de DPI:** o app usa a tabela do PAW3395SE. Mouses com outro sensor usam outra codificação, e gravar com a tabela errada resulta num DPI diferente do pedido.
- **Só KDE Plasma 6:** o widget é um plasmoid QML. A CLI funciona em qualquer Linux.
- **Só Linux x86_64 e aarch64:** a chamada `poll()` está declarada à mão para essas arquiteturas.
- **Leitura que pode estar desatualizada:** o widget não avisa quando a leitura está velha. Se o mouse descarregar até desligar, o ícone continua mostrando a última porcentagem lida.
- **Faixa do slider:** vai de 400 a 3200, cobrindo estágios comuns. Valores de 50 a 10 000 podem ser digitados.
- **Interface só em português.**
- **Projeto não oficial,** sem relação com VXE, ATK ou Compx. A gravação de DPI foi testada no hardware do autor; use por sua conta.

## Desinstalar

```bash
kpackagetool6 -t Plasma/Applet -r com.github.mats.vxe-r1
rm ~/.local/bin/vxe
sudo rm /etc/udev/rules.d/70-vxe-r1.rules && sudo udevadm control --reload
```

## Desenvolvimento

```bash
cargo test --manifest-path cli/Cargo.toml                                  # CLI
/usr/lib/qt6/bin/qmltestrunner -platform offscreen -input widget/tests     # lógica do widget
```

A estrutura é esta:

- `cli/`: o utilitário `vxe`;
- `widget/`: o plasmoid;
- `docs/protocol.md`: o protocolo levantado no hardware;
- `docs/superpowers/`: o design e os planos;
- `spike/probe.py`: o script de sondagem usado para descobrir o protocolo.

## Créditos

O protocolo foi entendido a partir das notas públicas do [atk-control](https://github.com/NasirA1991/atk-control) e do [OpenVXE](https://github.com/FrancescoLFM/OpenVXE), e depois verificado no hardware.
