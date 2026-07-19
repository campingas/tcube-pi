# Pi Admin HTTPS With Caddy

This deployment shape keeps `tcube-pi-admin` as a loopback-only HTTP service and uses Caddy as the HTTPS boundary for browser traffic.

## Process Boundary

- `tcube-pi-admin` listens on `127.0.0.1:8080`.
- Caddy listens on HTTPS for `tcube.local`, `10.55.0.1`, `localhost`, `127.0.0.1`, plus the LAN IP and `<hostname>.local` names added to `/etc/caddy/Caddyfile` by the release installer.
- Caddy also listens on plain HTTP port 80, where it serves only the root CA download at `/ca/root.crt` and redirects every other request to HTTPS.
- Caddy uses `tls internal`, so admin devices must trust Caddy's local root CA before browsers accept the certificate.
- `tcube-mdns-alias.service` publishes the `tcube.local` mDNS name through `avahi-publish` when the Pi hostname is not `tcube`.

## Install Packages

```sh
sudo apt update
sudo apt install -y caddy
```

## Install From GitHub Release Bundle

Release bundles are named like `tcube-pi-v0.1.0-linux-arm64.tar.gz` and target Raspberry Pi OS Lite 64-bit on Pi Zero 2 W. They include Linux arm64 Rust binaries, prebuilt admin UI assets, default content, Caddy/systemd files, and an installer.

Before pushing a release tag, prepare and commit matching manifest versions:

```sh
just prepare-release v0.1.0
git add Cargo.toml Cargo.lock admin-ui/package.json
git commit -m "chore: prepare release v0.1.0"
git tag -a v0.1.0 -m "v0.1.0"
git push origin main
git push origin v0.1.0
```

The release workflow fails if the tag version does not match both `Cargo.toml` and `admin-ui/package.json`.

On the Pi:

```sh
sha256sum -c SHA256SUMS
tar -xzf tcube-pi-v0.1.0-linux-arm64.tar.gz
cd tcube-pi-v0.1.0-linux-arm64
sudo ./install.sh
```

The installer writes files under `/opt/tcube`, `/etc/tcube`, and `/etc/systemd/system`, adds the current detected Pi LAN IP and `<hostname>.local` to `/etc/caddy/Caddyfile` when available, then enables `tcube-pi-admin` and Caddy. It also exports Caddy's internal root CA to `/opt/tcube/ca/root.crt`, enables `tcube-mdns-alias.service` when the hostname is not `tcube` and `avahi-publish` is available, and prints per-platform certificate trust instructions. It does not install Debian packages, so install `caddy` (and `avahi-utils` for the `tcube.local` alias) before running it.

## Install Files

Build or copy the `tcube-pi-admin` binary to:

```text
/opt/tcube/bin/tcube-pi-admin
```

Create the runtime user and writable data directories:

```sh
sudo useradd --system --home /var/lib/tcube --shell /usr/sbin/nologin tcube || true
sudo install -d -o tcube -g tcube -m 0750 /var/lib/tcube /var/lib/tcube/media
sudo install -d -o root -g root -m 0755 /opt/tcube/bin /opt/tcube/admin-ui /opt/tcube/content /etc/tcube
```

Copy the repository static UI and audio content into place:

```sh
just build-admin-ui
sudo cp -R admin-ui/build/. /opt/tcube/admin-ui/
sudo cp -R content/audio/. /opt/tcube/content/audio/
```

The Pi deployment serves prebuilt files from `admin-ui/build/`; Node and Bun are development-time tools only and are not required on the device. Use bun for every admin UI and JavaScript workflow.

Copy the service files:

```sh
sudo install -D -m 0644 deploy/pi-admin-caddy/tcube-pi-admin.env /etc/tcube/tcube-pi-admin.env
sudo install -D -m 0644 deploy/pi-admin-caddy/tcube-pi-admin.service /etc/systemd/system/tcube-pi-admin.service
sudo install -D -m 0644 deploy/pi-admin-caddy/Caddyfile /etc/caddy/Caddyfile
```

Then reload and start:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now tcube-pi-admin.service
sudo systemctl reload caddy
```

## Reach The Cube At tcube.local

`tcube.local` only resolves when something on the Pi announces it over mDNS. avahi-daemon announces `<hostname>.local` on its own, so the name works out of the box only when the Pi hostname is `tcube`.

For any other hostname, the release installer enables `tcube-mdns-alias.service`, which runs `/opt/tcube/bin/tcube-mdns-alias` to publish `tcube.local` for the current LAN address through `avahi-publish`. That command comes from the `avahi-utils` package, so install it before the release bundle:

```sh
sudo apt install -y avahi-utils
```

If the alias service cannot start, the installer still adds `<hostname>.local` to the Caddy site list, so `https://<hostname>.local/` and the LAN IP URL keep working.

## Trust The Local CA

On installed Pis, the installer exports Caddy's internal root certificate to `/opt/tcube/ca/root.crt`, and Caddy serves it to any device on the network at `http://<pi-address>/ca/root.crt`. The admin UI login screen also links to the same download with per-OS install steps.

Trust that certificate on each admin phone/laptop:

- macOS: `curl -fsSL http://<pi-address>/ca/root.crt -o ~/Downloads/tcube-root-ca.crt`, then `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/Downloads/tcube-root-ca.crt`.
- Linux: `sudo curl -fsSL http://<pi-address>/ca/root.crt -o /usr/local/share/ca-certificates/tcube-root-ca.crt`, then `sudo update-ca-certificates`.
- iPhone/iPad: open the URL in Safari, install the profile under Settings, General, VPN and Device Management, then enable full trust under Settings, General, About, Certificate Trust Settings.
- Android: download the file, then install it under Settings, Security, Encryption and credentials, Install a certificate, CA certificate.

Without that trust step, browsers will reject `https://tcube.local/`, `https://10.55.0.1/`, `https://localhost/`, `https://127.0.0.1/`, and configured LAN IP URLs.

For manual installs, Caddy stores its internal CA material under its application data directory. On Debian/Raspberry Pi OS package installs, inspect the current location with:

```sh
sudo caddy environ
```

On macOS local validation, Caddy may prompt for an administrator password while trying to install its internal root CA into the system trust store. It is acceptable to skip that local trust-store installation for a command-line smoke test and use `curl -k`, but real admin browser and phone testing must trust the Caddy root CA instead of bypassing certificate verification.

For local Mac browser testing, prefer `just run-pi-admin-lan-caddy` and open `https://127.0.0.1:8443/`. The `tcube.local` name only works when mDNS or a hosts-file entry resolves it to the machine running Caddy.

## Smoke Test

On the Pi:

```sh
curl http://127.0.0.1:8080/api/pi/v1/status
curl -k https://localhost/api/pi/v1/status
curl -k https://tcube.local/api/pi/v1/status
curl -k https://10.55.0.1/api/pi/v1/status
curl http://localhost/ca/root.crt
# If the release installer printed a detected Pi LAN URL:
curl -k https://<pi-lan-ip>/api/pi/v1/status
```

From an admin device that has trusted the root CA (no `-k`):

```sh
curl https://tcube.local/api/pi/v1/status
curl https://<pi-lan-ip>/api/pi/v1/status
```

If `tcube.local` does not resolve from the admin device, check `systemctl status tcube-mdns-alias --no-pager` and `avahi-resolve-host-name -4 tcube.local` on the Pi.

Use the `-k` command only for a quick Pi-local smoke test. Real admin browsers should trust the local CA instead of bypassing certificate verification.

## Browser Test Checklist

- Run `just run-pi-admin` and leave it open.
- For local development phone testing, run `just run-pi-admin-lan-caddy` in a second terminal and leave it open; it prints `https://127.0.0.1:8443/` for the same machine and `https://<mac-lan-ip>:8443/` for same-network phones.
- If the detected LAN IP is wrong, run `TCUBE_LAN_ADDRESS=<mac-lan-ip> just run-pi-admin-lan-caddy` and open the printed `https://<mac-lan-ip>:8443/` URL from the phone.
- `just run-pi-admin-caddy` remains available for port-443 local testing, but `just run-pi-admin-lan-caddy` is the preferred development path because it avoids privileged port 443 and disables Caddy's local admin endpoint for the dev process.
- From a phone on the same Wi-Fi/network, open the printed development URL `https://<mac-lan-ip>:8443/`; on an installed Pi, open `https://tcube.local/`, `https://10.55.0.1/`, or the installer-printed `https://<pi-lan-ip>/` URL.
- If you use the static deployment config instead, run `caddy run --config deploy/pi-admin-caddy/Caddyfile` and make sure `tcube.local` resolves to the machine running Caddy or that the target LAN IP is present in the Caddyfile site list.
- Direct backend smoke: open or curl `http://127.0.0.1:8080/api/pi/v1/status`.
- Caddy HTTPS smoke: open `https://localhost/` for local Mac browser testing.
- Confirm login, navigation, recording, upload, and generated-speech screens load through HTTPS.
- Confirm browser recording is tested through HTTPS or localhost; non-secure origins cannot use the microphone APIs.
- Confirm new `tcube-pi-admin` logs do not show invalid UTF-8 request-line errors.
- Forbidden URL: do not open `https://127.0.0.1:8080/`; port `8080` is plain HTTP for the Rust backend.
- Future Pi URL: use `https://tcube.local/` only after mDNS or a hosts-file entry resolves that name to the machine running Caddy.
