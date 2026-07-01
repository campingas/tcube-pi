set dotenv-load := true

default:
    just --list

check:
    cargo fmt --all --check
    cargo check --all-features
    cargo clippy --all-targets --all-features -- -D warnings

build:
    cargo build --workspace --all-features

build-release:
    cargo build --workspace --all-features --release

prepare-release version:
    deploy/pi-release/prepare-release {{version}}

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all-features

run-device-sim:
    cargo run --bin tcube-pi -- --backend sim

run-device-sim-audio:
    cargo run --bin tcube-pi -- --backend sim --audio local

run-device-pi:
    cargo run --bin tcube-pi -- --backend pi

run-pi-admin:
    #!/usr/bin/env bash
    set -euo pipefail

    lan_address=""
    if command -v hostname >/dev/null 2>&1; then
        lan_address="$(hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ { print; exit }' || true)"
    fi
    if [ -z "$lan_address" ] && command -v ip >/dev/null 2>&1; then
        lan_address="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }' || true)"
    fi
    if [ -z "$lan_address" ] && command -v ipconfig >/dev/null 2>&1; then
        for iface in en0 en1; do
            lan_address="$(ipconfig getifaddr "$iface" 2>/dev/null || true)"
            [ -n "$lan_address" ] && break
        done
    fi

    voxtral_api_base="${VOXTRAL_API_BASE:-https://127.0.0.1:11445}"
    vietnamese_vits_api_base="${VIETNAMESE_VITS_API_BASE:-https://127.0.0.1:11446}"
    speech_api_ca_cert="${TCUBE_SPEECH_API_CA_CERT:-}"
    if [ -z "$speech_api_ca_cert" ]; then
        for candidate in \
            "$HOME/Library/Application Support/Caddy/pki/authorities/local/root.crt" \
            "$HOME/.local/share/caddy/pki/authorities/local/root.crt" \
            "/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt"; do
            if [ -r "$candidate" ]; then
                speech_api_ca_cert="$candidate"
                break
            fi
        done
    fi
    export VOXTRAL_API_BASE="$voxtral_api_base"
    export VIETNAMESE_VITS_API_BASE="$vietnamese_vits_api_base"
    if [ -n "$speech_api_ca_cert" ]; then
        export TCUBE_SPEECH_API_CA_CERT="$speech_api_ca_cert"
    fi

    echo "T-Cube admin backend: http://127.0.0.1:8080/"
    echo "Phone/laptop URL via Caddy: https://tcube.local/"
    echo "USB gadget URL via Caddy: https://10.55.0.1/"
    if [ -n "$lan_address" ]; then
        echo "Detected host LAN address: $lan_address"
    fi
    echo "Run Caddy separately: caddy run --config deploy/pi-admin-caddy/Caddyfile"
    echo "For phone testing by LAN IP, run: just run-pi-admin-caddy"
    echo "Voxtral TTS API: $VOXTRAL_API_BASE"
    echo "Vietnamese VITS API: $VIETNAMESE_VITS_API_BASE"
    if [ -n "${TCUBE_SPEECH_API_CA_CERT:-}" ]; then
        echo "Speech API CA certificate: $TCUBE_SPEECH_API_CA_CERT"
    else
        echo "Speech API CA certificate: not configured; Caddy tls internal TTS endpoints may appear offline."
    fi
    echo

    cargo run --bin tcube-pi-admin -- --bind 127.0.0.1:8080 --database data/tcube.sqlite3 --ui-dist admin-ui/build --media-root data/audio --content-root content --hostname tcube.local --usb-address 10.55.0.1

run-pi-admin-caddy:
    #!/usr/bin/env bash
    set -euo pipefail

    lan_address=""
    if command -v hostname >/dev/null 2>&1; then
        lan_address="$(hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ { print; exit }' || true)"
    fi
    if [ -z "$lan_address" ] && command -v ip >/dev/null 2>&1; then
        lan_address="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }' || true)"
    fi
    if [ -z "$lan_address" ] && command -v ipconfig >/dev/null 2>&1; then
        for iface in en0 en1; do
            lan_address="$(ipconfig getifaddr "$iface" 2>/dev/null || true)"
            [ -n "$lan_address" ] && break
        done
    fi

    hosts="tcube.local, 10.55.0.1, localhost, 127.0.0.1"
    if [ -n "$lan_address" ]; then
        hosts="$hosts, $lan_address"
    fi

    caddy_config="$(mktemp "${TMPDIR:-/tmp}/tcube-pi-caddy.XXXXXX")"
    trap 'rm -f "$caddy_config"' EXIT
    cat >"$caddy_config" <<EOF
    $hosts {
        tls internal
        encode zstd gzip
        reverse_proxy 127.0.0.1:8080
    }
    EOF

    echo "Caddy phone/laptop URL: https://tcube.local/"
    if [ -n "$lan_address" ]; then
        echo "Caddy LAN IP URL: https://$lan_address/"
    fi
    echo "Phone browsers must trust Caddy's local root CA before HTTPS works cleanly."
    echo

    caddy run --config "$caddy_config" --adapter caddyfile

install-admin-ui:
    pnpm --dir admin-ui install

build-admin-ui:
    pnpm --dir admin-ui run build

check-admin-ui:
    pnpm --dir admin-ui run check

test-admin-ui-unit:
    pnpm --dir admin-ui run test:unit

test-admin-ui-mobile:
    pnpm --dir admin-ui run test:e2e --project=mobile

validate-pi-admin-caddy:
    caddy validate --config deploy/pi-admin-caddy/Caddyfile

measure-pi-admin button_presses='1000' admin_requests='600' admin_workers='4':
    cargo run --bin tcube-pi-admin-measure -- --base-url http://127.0.0.1:8080 --content content/content.json --button-presses {{button_presses}} --admin-requests {{admin_requests}} --admin-workers {{admin_workers}}
