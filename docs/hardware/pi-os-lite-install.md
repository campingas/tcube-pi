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
sudo apt install -y avahi-utils ca-certificates caddy curl git just sqlite3
```

Package purpose:

- `avahi-utils`: publishes the `tcube.local` mDNS alias when the Pi hostname is not `tcube`.
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

For low-level GPIO and MP3 diagnostics (optional; the Rust runtime needs neither), also install:

```sh
sudo apt install -y gpiod mpg123
```

Package purpose:

- `alsa-utils`: sound device inspection, mixer control, and command-line playback diagnostics.
- `gpiod`: `gpiomon` command-line GPIO edge detection for wiring diagnostics independent of the runtime.
- `mpg123`: command-line MP3 playback diagnostics independent of the runtime.

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

Do not install Node.js, Bun, Vite, Python model workers, or ML tooling on the Pi for normal operation. The admin UI is prebuilt into the release bundle, and speech/model workers are external development or Mac-side services.

## Enable I2S Audio

The release installer appends this MAX98357A I2S block to the boot config automatically when it is missing, and prints a reboot reminder:

```text
dtparam=i2s=on
dtoverlay=max98357a
```

To enable it manually before installing, add the same lines to `/boot/firmware/config.txt` (older images use `/boot/config.txt`). Reboot, then confirm the card appears:

```sh
sudo reboot
aplay -l
```

The expected prototype card name includes `sndrpimaxims` with card id `MAX98357A`. If no I2S card appears, power down and re-check the wiring and config before connecting or driving the speaker.

## Install Latest Release

Install the latest release bundle directly from GitHub Releases:

```sh
curl -fsSL https://raw.githubusercontent.com/campingas/tcube-pi/main/deploy/pi-release/install-latest | sudo bash
```

To install a specific version:

```sh
curl -fsSL https://raw.githubusercontent.com/campingas/tcube-pi/main/deploy/pi-release/install-latest | sudo env TCUBE_PI_VERSION=v0.0.3 bash
```

The same command also updates an existing installation. If the Pi already runs the resolved version (recorded in `/opt/tcube/VERSION`), the script exits early without downloading anything. To force a reinstall of the same version:

```sh
curl -fsSL https://raw.githubusercontent.com/campingas/tcube-pi/main/deploy/pi-release/install-latest | sudo env TCUBE_PI_FORCE=1 bash
```

On update, `tcube-pi-admin.service` and `tcube-pi.service` are restarted automatically when their binaries, unit files, env files, or (for the runtime) content pack changed, so the new version is live immediately. Existing `/etc/tcube/tcube-pi-admin.env` and `/etc/tcube/tcube-pi.env` files are preserved and the new release defaults are written to matching `.env.dist` files; data under `/var/lib/tcube` is never touched.

The bootstrap script downloads the selected release archive and `SHA256SUMS`, verifies the archive plus bundled installer and binaries, extracts the bundle in a temporary directory, then runs the bundled installer. The installer writes application files under `/opt/tcube`, configuration under `/etc/tcube`, data under `/var/lib/tcube`, and systemd service files under `/etc/systemd/system`. It adds the current detected Pi LAN IP and `<hostname>.local` to `/etc/caddy/Caddyfile` when available, then enables `tcube-pi-admin`, the `tcube-pi` child runtime, and Caddy. When the installer had to add the I2S overlay to the boot config it defers starting `tcube-pi.service` to the next boot and prints a reboot reminder; it also disables the temporary `tcube-button-smoke.service` if the bench payload was installed earlier.

The installer also wires up device trust and naming:

- It exports Caddy's internal root certificate to `/opt/tcube/ca/root.crt`, which Caddy serves to the network at `http://<pi-address>/ca/root.crt`.
- It enables `tcube-mdns-alias.service` so `https://tcube.local/` resolves even when the Pi hostname is not `tcube`; this needs the `avahi-utils` base package.
- It prints per-platform steps for trusting the cube certificate on macOS, Linux, iPhone/iPad, and Android admin devices.

## Wi-Fi Persistence Safeguard

Pi Imager or first-boot cloud-init connectivity proves that the current boot can reach the network, but it does not by itself prove that NetworkManager has a keyfile that survives reboot. A profile can remain active from `/run/NetworkManager/system-connections/`, which is temporary state.

Before any T-Cube application files are written, the installer asks NetworkManager for the connected `wlan0` profile's name, UUID, type, and backing filename. If the filename is already under `/etc/NetworkManager/system-connections/`, the installer leaves the profile untouched. If the filename is temporary, it clones the active UUID through `nmcli` as `tcube-wifi`, enables autoconnect with priority `100`, and verifies the resulting keyfile is under `/etc/NetworkManager/system-connections/`, owned by root, and mode `600`. It does not activate or reconnect the clone during installation, so the working connection remains in place.

If `nmcli` is unavailable or `wlan0` is not connected, the guard skips cleanly. If an unrelated profile named `tcube-wifi` already exists, or cloning and validation fail, the installer aborts before application writes and does not print connection secrets.

For a `tcube-wifi` name collision, keep the current connection active and inspect only non-secret metadata:

```sh
sudo nmcli -f NAME,UUID,TYPE,FILENAME connection show
```

If review confirms that profile is unrelated and should be retained, give it a distinct name without activating or disconnecting it, then rerun the installer:

```sh
sudo nmcli connection modify id tcube-wifi connection.id tcube-wifi-existing
```

Do not delete a profile until another persistent WLAN profile has passed the reboot/reconnect gate below.

## Post-Install Checks

Reboot is the acceptance gate for both Wi-Fi persistence and the installed journald policy. After the installer finishes, reboot the Pi, reconnect through the network without re-entering Wi-Fi credentials, and confirm that the active WLAN is now backed by persistent storage:

```sh
sudo reboot
nmcli -f NAME,UUID,TYPE,FILENAME connection show --active
sudo stat -c '%U %a %n' /etc/NetworkManager/system-connections/*.nmconnection
```

The active WLAN filename must be under `/etc/NetworkManager/system-connections/`, and its keyfile must report owner `root` and mode `600`. A successful install without a successful reboot and reconnect is not accepted as persistent connectivity.

The installer writes `/etc/systemd/journald.conf.d/tcube-persistent.conf` with `Storage=persistent` and `SystemMaxUse=64M`. After reboot, verify that the current and previous boots are available and inspect disk use:

```sh
journalctl --list-boots
journalctl -b -u tcube-pi --no-pager
journalctl -b -u tcube-pi-admin --no-pager
journalctl --disk-usage
```

Check the services:

```sh
systemctl status tcube-pi --no-pager
systemctl status tcube-pi-admin --no-pager
systemctl status caddy --no-pager
```

Check the child runtime and press a button:

```sh
journalctl -u tcube-pi -f
```

Press the button wired to BCM GPIO17 (button 1); the speaker plays the mapped content and the journal logs the press. If your buttons use different lines, edit `TCUBE_PI_BUTTON_GPIOS` in `/etc/tcube/tcube-pi.env` and run `sudo systemctl restart tcube-pi`.

Check the loopback backend and Caddy HTTPS boundary:

```sh
curl http://127.0.0.1:8080/api/pi/v1/status
curl -k https://localhost/api/pi/v1/status
curl -k https://<pi-lan-ip>/api/pi/v1/status
curl http://localhost/ca/root.crt
```

Check that `tcube.local` is announced over mDNS:

```sh
systemctl status tcube-mdns-alias --no-pager
avahi-resolve-host-name -4 tcube.local
```

Use `curl -k` only for a Pi-local smoke test. Real admin browsers and phones must trust Caddy's internal root CA before using `https://tcube.local/`, `https://10.55.0.1/`, or local LAN HTTPS URLs.

Trust the cube certificate on each admin device by downloading `http://<pi-address>/ca/root.crt` and marking it trusted; the installer output and the admin UI login screen both list the per-platform steps. After trusting it, verify without `-k` from the admin device:

```sh
curl https://tcube.local/api/pi/v1/status
curl https://<pi-lan-ip>/api/pi/v1/status
```

## Admin Access

Open the admin UI through Caddy, not the loopback Rust service:

- Pi-local browser or SSH tunnel smoke: `https://localhost/`
- Home-network browser: `https://tcube.local/` (announced by avahi directly or by `tcube-mdns-alias.service`)
- Home-network browser fallback: `https://<hostname>.local/` when the alias service is unavailable
- USB gadget path when configured: `https://10.55.0.1/`
- Home-network browser by IP when the release installer detected and added the Pi's current LAN IP to Caddy: `https://<pi-lan-ip>/`

Do not open `https://127.0.0.1:8080/`; port `8080` is plain HTTP and intended only for the local Caddy reverse proxy.
