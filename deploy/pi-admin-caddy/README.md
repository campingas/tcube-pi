# Pi Admin HTTPS With Caddy

This deployment shape keeps `tcube-pi-admin` as a loopback-only HTTP service and uses Caddy as the HTTPS boundary for browser traffic.

## Process Boundary

- `tcube-pi-admin` listens on `127.0.0.1:8080`.
- Caddy listens on HTTPS for `tcube.local`, `10.55.0.1`, `localhost`, and `127.0.0.1`.
- Caddy uses `tls internal`, so admin devices must trust Caddy's local root CA before browsers accept the certificate.

## Install Packages

```sh
sudo apt update
sudo apt install -y caddy
```

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

The Pi deployment serves prebuilt files from `admin-ui/build/`; Node and pnpm are development-time tools only and are not required on the device. Use pnpm for every admin UI and JavaScript workflow.

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

## Trust The Local CA

Caddy stores its internal CA material under its application data directory. On Debian/Raspberry Pi OS package installs, inspect the current location with:

```sh
sudo caddy environ
```

Export or copy Caddy's internal root certificate to each admin phone/laptop and mark it trusted for websites. Without that trust step, browsers will reject `https://tcube.local/`, `https://10.55.0.1/`, `https://localhost/`, and `https://127.0.0.1/`.

On macOS local validation, Caddy may prompt for an administrator password while trying to install its internal root CA into the system trust store. It is acceptable to skip that local trust-store installation for a command-line smoke test and use `curl -k`, but real admin browser and phone testing must trust the Caddy root CA instead of bypassing certificate verification.

For local Mac browser testing, prefer `https://localhost/`. The `tcube.local` name only works when mDNS or a hosts-file entry resolves it to the machine running Caddy.

## Smoke Test

```sh
curl http://127.0.0.1:8080/api/pi/v1/status
curl -k https://localhost/api/pi/v1/status
curl -k https://tcube.local/api/pi/v1/status
curl -k https://10.55.0.1/api/pi/v1/status
```

Use the `-k` command only for a quick Pi-local smoke test. Real admin browsers should trust the local CA instead of bypassing certificate verification.

## Browser Test Checklist

- Run `just run-pi-admin` and leave it open.
- Run `caddy run --config deploy/pi-admin-caddy/Caddyfile` and leave it open.
- Direct backend smoke: open or curl `http://127.0.0.1:8080/api/pi/v1/status`.
- Caddy HTTPS smoke: open `https://localhost/` for local Mac browser testing.
- Confirm login, navigation, recording, upload, and generated-speech screens load through HTTPS.
- Confirm browser recording is tested through HTTPS or localhost; non-secure origins cannot use the microphone APIs.
- Confirm new `tcube-pi-admin` logs do not show invalid UTF-8 request-line errors.
- Forbidden URL: do not open `https://127.0.0.1:8080/`; port `8080` is plain HTTP for the Rust backend.
- Future Pi URL: use `https://tcube.local/` only after mDNS or a hosts-file entry resolves that name to the machine running Caddy.
