# mouse-battery-levels

A minimal **KDE Plasma 6** widget that shows the battery level of the **VXE R1 S NK** mouse in your panel and lets you change its DPI. It is backed by `vxe`, a tiny Rust command-line tool with no dependencies.

- **Panel icon:** a mouse outline that fills up with the battery level, with the percentage next to it. It is green at 40% and above, yellow from 15% to 39%, and red below 15%. A small bolt appears while it charges.
- **Popup:** opens when you click the icon. It shows the battery, the connection (2.4 GHz or wired) and a DPI slider. Click the DPI number to type a value.
- **Lightweight:** nothing keeps running in the background. The widget runs `vxe status` once a minute, and each read takes about 20 ms.
- **Never wakes the mouse:** the tool first asks the receiver whether the mouse is awake, and only then asks for the battery. While the mouse sleeps, the widget keeps showing the last reading.

## Why

The kernel doesn't expose this mouse's battery under `/sys/class/power_supply`, so UPower and Plasma's battery applet can't show it. The in-kernel `hid-kysona` driver only matches other receivers. This project talks to the receiver directly over the Compx/ATK HID configuration protocol, which is documented in [`docs/protocol.md`](docs/protocol.md) (in Portuguese).

## Requirements

- KDE Plasma 6 (tested on 6.8, Wayland)
- Rust (`cargo`) to build the CLI
- `kpackagetool6`, which ships with Plasma
- `sudo` once, to install the udev rule

## Install

```bash
git clone https://github.com/maats01/mouse-battery-levels.git
cd mouse-battery-levels
./install.sh
```

`install.sh` does three things:

1. builds `vxe` and copies it to `~/.local/bin/vxe`;
2. installs or updates the **"VXE R1"** widget for your user;
3. installs `/etc/udev/rules.d/70-vxe-r1.rules`, which lets the logged-in user access the mouse without `sudo`. It only asks for your password when the rule is missing or out of date.

Then right-click your panel, choose **Add Widgets…**, search for **VXE R1** and drag it next to the system tray. Put it on the panel itself rather than inside the tray: tray icons are forced square, which leaves no room for the percentage.

If an update doesn't show up, restart the panel:

```bash
systemctl --user restart plasma-plasmashell
```

## Usage

### Widget

- **Battery:** shown on the icon and in the popup. `––` means there has been no reading yet, or neither the receiver nor the cable is plugged in.
- **DPI slider:** drag between 400 and 3200 in steps of 50. The value is written only when you release the slider.
- **Typed DPI:** click the number and type any value from 50 to 10 000. **Enter** applies it and **Esc** cancels. Values that aren't a multiple of 50 are rounded.
- **Dots on the slider:** they mark the mouse's other DPI stages.

### Command line

```bash
vxe status
# {"connected":true,"mode":"wireless","battery":45,"charging":false,"dpi":1200,"stages":[800,1200,1600,3200]}

vxe dpi 1600
# writes 1600 to the active stage and prints the updated status
```

| Output | Meaning |
|---|---|
| `{"connected":false}` | mouse asleep, off or unreachable; nothing was sent to it |
| `{"error":"no_receiver"}` | neither the receiver nor the cable is plugged in |
| `{"error":"permission"}` | no access to the hidraw device; run `./install.sh` |
| `{"error":"write_failed"}` | the mouse didn't confirm the DPI write |
| `{"error":"io"}` | invalid reply, or unexpected content in the mouse's memory |

Exit codes are `0` for success or a sleeping mouse, `1` for an error and `2` for an invalid argument.

## How the DPI change works

The protocol has **no "current DPI" command**. DPI only exists as the stages stored in the mouse's memory, the same ones the button under the mouse cycles through. So `vxe dpi N` **rewrites the value of the active stage**:

- if stage 2 was 1200 and you pick 1000, stage 2 becomes 1000;
- the DPI button keeps cycling through the stages as before;
- the new value survives switching the mouse off and on.

Safety measures:

- **Backup:** before the first write, the original stages are saved to `~/.local/state/vxe/backup.hex`. This file is never overwritten.
- **Validation:** `vxe` refuses to write unless the stage header and every stage record are valid, and it writes only the 4 bytes of the active stage.
- **Confirmation:** every write must be acknowledged by the mouse and is read back. A mismatch is reported as `write_failed`.
- **Sleeping mouse:** nothing is written while the mouse sleeps.

## Limitations

### The battery reading can be out of date, with no warning

This mouse goes to sleep after 30 seconds without movement. While it sleeps, the receiver reports it as offline, and `vxe` deliberately doesn't ask it for the battery, so that it never wakes the mouse. Instead, the widget keeps showing the **last level it read while the mouse was awake**, with the same icon and colours as a fresh reading. This is on purpose: otherwise the icon would show `––` every time you stop moving the mouse for half a minute, for example while typing.

The downside is that `vxe` can't tell a sleeping mouse from one that is switched off, out of range, or completely flat. All of these show up as `{"connected":false}`. So:

- **If the battery runs out,** the icon keeps showing the last percentage it saw (say, 3% in red) until the mouse is charged and used again. It never switches to an "empty" or "off" state.
- **If you switch the mouse off or take it elsewhere,** the icon still shows its last level.
- **There is no "last updated" time.** Nothing in the icon or popup tells you how old the reading is.
- **After a restart** of the panel or a new login, the widget has no previous reading, so it shows `––` until you move the mouse and the next check runs (up to one minute).

In practice, the level only drops while you are using the mouse, and that is exactly when the widget gets fresh readings. By the time the mouse dies, the icon has been red (below 15%) for a while.

### Other limitations

- **Tested with one mouse only:** the VXE R1 S NK (CID/MID `2,117`, PAW3395SE sensor), through its NearLink receiver (`373b:10c9`) and over the cable (`373b:121f`). Other Compx/ATK receivers and mice probably speak the same protocol but aren't recognised. The IDs live in `cli/src/device.rs` and `udev/70-vxe-r1.rules`.
- **DPI encoding:** the PAW3395SE code table is used. Mice with a different sensor encode DPI differently, so writing with the wrong table sets a different DPI from the one you asked for.
- **KDE Plasma 6 only:** the widget is a QML plasmoid. The `vxe` CLI works on any Linux.
- **x86_64 and aarch64 only:** the `poll()` system call is declared by hand for these architectures.
- **Slider range:** the slider covers 400–3200, which fits typical stages. Values from 50 to 10 000 can still be typed.
- **Unofficial:** this project is not affiliated with VXE, ATK or Compx. DPI writing was tested on the author's hardware; use it at your own risk.

## Uninstall

```bash
kpackagetool6 -t Plasma/Applet -r com.github.mats.vxe-r1
rm ~/.local/bin/vxe
sudo rm /etc/udev/rules.d/70-vxe-r1.rules && sudo udevadm control --reload
```

## Development

```bash
cargo test --manifest-path cli/Cargo.toml                                  # CLI
/usr/lib/qt6/bin/qmltestrunner -platform offscreen -input widget/tests     # widget logic
```

Layout:

- `cli/`: the `vxe` tool;
- `widget/`: the plasmoid;
- `docs/protocol.md`: the protocol as observed on hardware;
- `docs/superpowers/`: design and plans (in Portuguese);
- `spike/probe.py`: the probe script used to discover the protocol.

## Credits

The protocol was worked out from the public notes of [atk-control](https://github.com/NasirA1991/atk-control) and [OpenVXE](https://github.com/FrancescoLFM/OpenVXE), then verified on hardware.

## License

[MIT](LICENSE).
