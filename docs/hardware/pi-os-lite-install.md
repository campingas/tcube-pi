# Raspberry Pi OS Lite Install

This guide prepares a fresh Raspberry Pi OS Lite 64-bit install on a Raspberry Pi Zero 2 W, installs the latest T-Cube Pi release bundle, and verifies that the Pi admin service works behind Caddy HTTPS.

Use [Hardware Assembly](hardware-assembly.md) for the physical component inventory, wiring, and bring-up order.

## Target

- Raspberry Pi Zero 2 W
- Raspberry Pi OS Lite 64-bit
- Network access for package installation and GitHub Release downloads
- SSH access or a directly attached keyboard and display for first setup

## Base Packages

Install the packages required by the release installer and Pi-hosted admin service:

```sh
sudo apt update
sudo apt install -y ca-certificates caddy curl git just sqlite3
```

Package purpose:

- `ca-certificates`: TLS certificate roots for package and release downloads.
- `caddy`: HTTPS reverse proxy for the local admin UI and API.
- `curl`: release download helper.
- `git`: source checkout when developing on the Pi.
- `just`: documented project command runner.
- `sqlite3`: local database inspection and maintenance.

## Hardware Diagnostics

Install ALSA tools before validating the MAX98357A amplifier and speaker:

```sh
sudo apt install -y alsa-utils
```

For the temporary one-button smoke payload under `deploy/pi-zero-button-smoke`, also install:

```sh
sudo apt install -y gpiod mpg123
```

Package purpose:

- `alsa-utils`: sound device inspection, mixer control, and command-line playback diagnostics.
- `gpiod`: `gpiomon` command-line GPIO edge detection for the temporary physical-button smoke test.
- `mpg123`: command-line MP3 playback for stock music content while the final Rust Pi GPIO backend is pending.

## Optional Native Build Tools

The recommended Pi install path is the GitHub Release bundle. Install native build tools only when building or validating the Rust workspace directly on the Pi:

```sh
sudo apt install -y build-essential pkg-config
```

Install Rust with the official toolchain installer unless the project later pins another Pi-specific method:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then validate from a source checkout:

```sh
just check
just test
```

Do not install Node.js, pnpm, Vite, Python model workers, or ML tooling on the Pi for normal operation. The admin UI is prebuilt into the release bundle, and speech/model workers are external development or Mac-side services.

## Enable I2S Audio

For the MAX98357A prototype output path, enable I2S in `/boot/config.txt`:

```text
dtparam=i2s=on
dtoverlay=max98357a
```

Reboot, then confirm the card appears:

```sh
sudo reboot
aplay -l
```

The expected prototype card name includes `sndrpimaxims`. If no I2S card appears, power down and re-check the wiring and config before connecting or driving the speaker.

## Install Latest Release

Download the latest release bundle from GitHub Releases:

```sh
repo="campingas/tcube-pi"
tag="$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)"
curl -fLO "https://github.com/${repo}/releases/download/${tag}/tcube-pi-${tag}-linux-arm64.tar.gz"
curl -fLO "https://github.com/${repo}/releases/download/${tag}/SHA256SUMS"
grep "tcube-pi-${tag}-linux-arm64.tar.gz$" SHA256SUMS | sha256sum -c -
tar -xzf "tcube-pi-${tag}-linux-arm64.tar.gz"
cd "tcube-pi-${tag}-linux-arm64"
grep -E ' (bin/|install\.sh$)' ../SHA256SUMS | sha256sum -c -
sudo ./install.sh
```

The installer writes application files under `/opt/tcube`, configuration under `/etc/tcube`, data under `/var/lib/tcube`, and systemd service files under `/etc/systemd/system`. It enables `tcube-pi-admin` and Caddy.

## Post-Install Checks

Check the services:

```sh
systemctl status tcube-pi-admin --no-pager
systemctl status caddy --no-pager
```

Check the loopback backend and Caddy HTTPS boundary:

```sh
curl http://127.0.0.1:8080/api/pi/v1/status
curl -k https://localhost/api/pi/v1/status
```

Use `curl -k` only for a Pi-local smoke test. Real admin browsers and phones must trust Caddy's internal root CA before using `https://tcube.local/`, `https://10.55.0.1/`, or local LAN HTTPS URLs.

## Admin Access

Open the admin UI through Caddy, not the loopback Rust service:

- Pi-local browser or SSH tunnel smoke: `https://localhost/`
- Home-network browser after name resolution is configured: `https://tcube.local/`
- USB gadget path when configured: `https://10.55.0.1/`

Do not open `https://127.0.0.1:8080/`; port `8080` is plain HTTP and intended only for the local Caddy reverse proxy.
