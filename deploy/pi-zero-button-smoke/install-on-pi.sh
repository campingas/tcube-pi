#!/usr/bin/env bash
set -euo pipefail

payload_root="${1:-}"

if [[ -z "$payload_root" ]]; then
  echo "usage: sudo $0 /path/to/tcube-pi-payload" >&2
  exit 2
fi

if [[ "$(id -u)" -ne 0 ]]; then
  echo "run with sudo so files can be installed under /opt, /etc, and /boot/firmware" >&2
  exit 2
fi

script_dir="$payload_root/deploy/pi-zero-button-smoke"
content_dir="$payload_root/content"

if [[ ! -d "$script_dir" ]]; then
  echo "missing payload script directory: $script_dir" >&2
  exit 1
fi

if [[ ! -d "$content_dir" ]]; then
  echo "missing content directory: $content_dir" >&2
  exit 1
fi

apt-get update
apt-get install -y ca-certificates alsa-utils gpiod mpg123

install -d -m 0755 /opt/tcube/bin /opt/tcube/content /etc/tcube
cp -R "$content_dir/." /opt/tcube/content/
install -m 0755 "$script_dir/tcube-button-smoke" /opt/tcube/bin/tcube-button-smoke
install -m 0644 "$script_dir/tcube-button-smoke.env" /etc/tcube/tcube-button-smoke.env
install -m 0644 "$script_dir/tcube-button-smoke.service" /etc/systemd/system/tcube-button-smoke.service

boot_config="/boot/firmware/config.txt"
if [[ -f "$boot_config" ]] && ! grep -q "T-Cube MAX98357A I2S audio" "$boot_config"; then
  cat >> "$boot_config" <<'CONFIG'

# T-Cube MAX98357A I2S audio
dtparam=i2s=on
dtoverlay=max98357a
CONFIG
fi

systemctl daemon-reload
systemctl enable tcube-button-smoke.service

echo "Installed T-Cube button smoke payload."
echo "Edit /etc/tcube/tcube-button-smoke.env if your button is not on BCM GPIO17 or the MAX98357A card has a different ALSA name."
echo "Reboot before testing I2S audio: sudo reboot"
