#!/usr/bin/env bash
# Build and install the VXE R1 widget for the current user.
set -euo pipefail
cd "$(dirname "$0")"

echo "==> Building vxe"
cargo build --release --manifest-path cli/Cargo.toml
mkdir -p "$HOME/.local/bin"
install -m755 cli/target/release/vxe "$HOME/.local/bin/vxe"

echo "==> Installing the widget"
kpackagetool6 -t Plasma/Applet -u widget 2>/dev/null || kpackagetool6 -t Plasma/Applet -i widget

rule=/etc/udev/rules.d/70-vxe-r1.rules
if ! cmp -s udev/70-vxe-r1.rules "$rule"; then
    echo "==> Installing the udev rule (asks for sudo once)"
    sudo install -m644 udev/70-vxe-r1.rules "$rule"
    sudo udevadm control --reload
    sudo udevadm trigger
fi

echo "==> Done: $("$HOME/.local/bin/vxe" status || true)"
echo "Add the \"VXE R1\" widget to your panel (Add Widgets…), next to the system tray."
