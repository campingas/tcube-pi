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
    cargo run --bin tcube-pi --all-features -- --backend pi

run-pi-admin:
    #!/usr/bin/env bash
    set -euo pipefail

    detect_lan_address() {
        if [ -n "${TCUBE_LAN_ADDRESS:-}" ]; then
            printf '%s\n' "$TCUBE_LAN_ADDRESS"
            return
        fi

        if command -v hostname >/dev/null 2>&1; then
            candidate="$(hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ && $1 !~ /^127\./ && $1 !~ /^169\.254\./ { print; exit }' || true)"
            if [ -n "$candidate" ]; then
                printf '%s\n' "$candidate"
                return
            fi
        fi
        if command -v ip >/dev/null 2>&1; then
            candidate="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }' || true)"
            if [ -n "$candidate" ]; then
                printf '%s\n' "$candidate"
                return
            fi
        fi
        if command -v route >/dev/null 2>&1 && command -v ipconfig >/dev/null 2>&1; then
            iface="$(route -n get default 2>/dev/null | awk '/interface:/ { print $2; exit }' || true)"
            if [ -n "$iface" ]; then
                candidate="$(ipconfig getifaddr "$iface" 2>/dev/null || true)"
                if [ -n "$candidate" ]; then
                    printf '%s\n' "$candidate"
                    return
                fi
            fi
        fi
        if command -v ifconfig >/dev/null 2>&1; then
            ifconfig 2>/dev/null | awk '/^[a-zA-Z0-9]/ { iface=$1; sub(":", "", iface) } /inet / && iface != "lo0" && $2 !~ /^127\./ && $2 !~ /^169\.254\./ { print $2; exit }'
        fi
    }

    lan_address="$(detect_lan_address || true)"

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
    echo "This command starts the loopback HTTP backend only; browsers should use Caddy HTTPS."
    echo
    echo "Development Caddy command for browser/phone testing:"
    echo "  just run-pi-admin-lan-caddy"
    echo "Development admin UI after Caddy starts:"
    echo "  Same machine: https://127.0.0.1:8443/"
    if [ -n "$lan_address" ]; then
        echo "  Same Wi-Fi phone/laptop: https://$lan_address:8443/"
        echo "Detected host LAN address: $lan_address"
    else
        echo "  Same Wi-Fi phone/laptop: set TCUBE_LAN_ADDRESS=<host-lan-ip> and use https://<host-lan-ip>:8443/"
    fi
    echo "Installed Pi Caddy URLs use port 443: https://tcube.local/, https://10.55.0.1/, or the configured Pi LAN IP."
    echo "If LAN IP detection is wrong, run: TCUBE_LAN_ADDRESS=<host-lan-ip> just run-pi-admin-lan-caddy"
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

    list_host_ipv4_addresses() {
        if command -v ifconfig >/dev/null 2>&1; then
            ifconfig 2>/dev/null | awk '/inet / && $2 !~ /^127\./ && $2 !~ /^169\.254\./ { print $2 }'
        fi
    }

    detect_lan_address() {
        if [ -n "${TCUBE_LAN_ADDRESS:-}" ]; then
            printf '%s\n' "$TCUBE_LAN_ADDRESS"
            return
        fi

        if command -v hostname >/dev/null 2>&1; then
            candidate="$(hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ && $1 !~ /^127\./ && $1 !~ /^169\.254\./ { print; exit }' || true)"
            if [ -n "$candidate" ]; then
                printf '%s\n' "$candidate"
                return
            fi
        fi
        if command -v ip >/dev/null 2>&1; then
            candidate="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }' || true)"
            if [ -n "$candidate" ]; then
                printf '%s\n' "$candidate"
                return
            fi
        fi
        if command -v route >/dev/null 2>&1 && command -v ipconfig >/dev/null 2>&1; then
            iface="$(route -n get default 2>/dev/null | awk '/interface:/ { print $2; exit }' || true)"
            if [ -n "$iface" ]; then
                candidate="$(ipconfig getifaddr "$iface" 2>/dev/null || true)"
                if [ -n "$candidate" ]; then
                    printf '%s\n' "$candidate"
                    return
                fi
            fi
        fi
        if command -v ifconfig >/dev/null 2>&1; then
            ifconfig 2>/dev/null | awk '/^[a-zA-Z0-9]/ { iface=$1; sub(":", "", iface) } /inet / && iface != "lo0" && $2 !~ /^127\./ && $2 !~ /^169\.254\./ { print $2; exit }'
        fi
    }

    lan_address="$(detect_lan_address || true)"
    if [ -n "$lan_address" ] && command -v ifconfig >/dev/null 2>&1 && ! list_host_ipv4_addresses | grep -Fxq "$lan_address"; then
        echo "LAN IP $lan_address is not assigned to this host."
        echo "Detected host IPv4 addresses:"
        list_host_ipv4_addresses | sed 's/^/  /'
        exit 1
    fi

    hosts="tcube.local, 10.55.0.1, localhost, 127.0.0.1"
    if [ -n "$lan_address" ]; then
        hosts="$hosts, $lan_address"
    fi

    caddy_config="$(mktemp "${TMPDIR:-/tmp}/tcube-pi-caddy.XXXXXX")"
    trap 'rm -f "$caddy_config"' EXIT
    cat >"$caddy_config" <<EOF
    {
        admin off
    }

    $hosts {
        bind 0.0.0.0
        tls internal
        encode zstd gzip
        reverse_proxy 127.0.0.1:8080
    }
    EOF

    echo "Caddy port-443 URL: https://tcube.local/"
    echo "Same-machine smoke URL: https://127.0.0.1/"
    if [ -n "$lan_address" ]; then
        echo "Caddy LAN IP URL: https://$lan_address/"
    else
        echo "Caddy LAN IP URL: not detected"
        echo "Set it explicitly with: TCUBE_LAN_ADDRESS=<mac-lan-ip> just run-pi-admin-caddy"
    fi
    echo "If port 443 is blocked or awkward locally, use: just run-pi-admin-lan-caddy"
    echo "Phone browsers must trust Caddy's local root CA before HTTPS works cleanly."
    echo

    caddy run --config "$caddy_config" --adapter caddyfile

run-pi-admin-lan-caddy port='8443':
    #!/usr/bin/env bash
    set -euo pipefail

    list_host_ipv4_addresses() {
        if command -v ifconfig >/dev/null 2>&1; then
            ifconfig 2>/dev/null | awk '/inet / && $2 !~ /^127\./ && $2 !~ /^169\.254\./ { print $2 }'
        fi
    }

    detect_lan_address() {
        if [ -n "${TCUBE_LAN_ADDRESS:-}" ]; then
            printf '%s\n' "$TCUBE_LAN_ADDRESS"
            return
        fi

        if command -v hostname >/dev/null 2>&1; then
            candidate="$(hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ && $1 !~ /^127\./ && $1 !~ /^169\.254\./ { print; exit }' || true)"
            if [ -n "$candidate" ]; then
                printf '%s\n' "$candidate"
                return
            fi
        fi
        if command -v ip >/dev/null 2>&1; then
            candidate="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }' || true)"
            if [ -n "$candidate" ]; then
                printf '%s\n' "$candidate"
                return
            fi
        fi
        if command -v route >/dev/null 2>&1 && command -v ipconfig >/dev/null 2>&1; then
            iface="$(route -n get default 2>/dev/null | awk '/interface:/ { print $2; exit }' || true)"
            if [ -n "$iface" ]; then
                candidate="$(ipconfig getifaddr "$iface" 2>/dev/null || true)"
                if [ -n "$candidate" ]; then
                    printf '%s\n' "$candidate"
                    return
                fi
            fi
        fi
        if command -v ifconfig >/dev/null 2>&1; then
            ifconfig 2>/dev/null | awk '/^[a-zA-Z0-9]/ { iface=$1; sub(":", "", iface) } /inet / && iface != "lo0" && $2 !~ /^127\./ && $2 !~ /^169\.254\./ { print $2; exit }'
        fi
    }

    lan_address="$(detect_lan_address || true)"
    if [ -z "$lan_address" ]; then
        echo "Could not detect a LAN IP."
        echo "Run again with: TCUBE_LAN_ADDRESS=<mac-lan-ip> just run-pi-admin-lan-caddy {{port}}"
        exit 1
    fi
    if command -v ifconfig >/dev/null 2>&1 && ! list_host_ipv4_addresses | grep -Fxq "$lan_address"; then
        echo "LAN IP $lan_address is not assigned to this host."
        echo "Detected host IPv4 addresses:"
        list_host_ipv4_addresses | sed 's/^/  /'
        exit 1
    fi

    caddy_config="$(mktemp "${TMPDIR:-/tmp}/tcube-pi-lan-caddy.XXXXXX")"
    trap 'rm -f "$caddy_config"' EXIT
    cat >"$caddy_config" <<EOF
    {
        admin off
    }

    https://$lan_address:{{port}}, https://127.0.0.1:{{port}}, https://localhost:{{port}}, https://tcube.local:{{port}}, https://:{{port}} {
        bind 0.0.0.0
        tls internal
        encode zstd gzip
        reverse_proxy 127.0.0.1:8080
    }
    EOF

    echo "Caddy LAN URL: https://$lan_address:{{port}}/"
    echo "Local smoke URL: https://127.0.0.1:{{port}}/"
    echo "If the phone still cannot connect, check macOS Firewall and Wi-Fi client isolation."
    echo

    caddy run --config "$caddy_config" --adapter caddyfile

debug-pi-admin-lan port='443':
    #!/usr/bin/env bash
    set -euo pipefail

    list_host_ipv4_addresses() {
        if command -v ifconfig >/dev/null 2>&1; then
            ifconfig 2>/dev/null | awk '/inet / && $2 !~ /^127\./ && $2 !~ /^169\.254\./ { print $2 }'
        fi
    }

    detect_lan_address() {
        if [ -n "${TCUBE_LAN_ADDRESS:-}" ]; then
            printf '%s\n' "$TCUBE_LAN_ADDRESS"
            return
        fi

        if command -v route >/dev/null 2>&1 && command -v ipconfig >/dev/null 2>&1; then
            iface="$(route -n get default 2>/dev/null | awk '/interface:/ { print $2; exit }' || true)"
            if [ -n "$iface" ]; then
                candidate="$(ipconfig getifaddr "$iface" 2>/dev/null || true)"
                if [ -n "$candidate" ]; then
                    printf '%s\n' "$candidate"
                    return
                fi
            fi
        fi
        if command -v hostname >/dev/null 2>&1; then
            hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ && $1 !~ /^127\./ && $1 !~ /^169\.254\./ { print; exit }'
        fi
    }

    lan_address="$(detect_lan_address || true)"
    echo "Detected LAN IP: ${lan_address:-not detected}"
    if [ -n "$lan_address" ] && command -v ifconfig >/dev/null 2>&1 && ! list_host_ipv4_addresses | grep -Fxq "$lan_address"; then
        echo "Warning: $lan_address is not assigned to this host."
    fi
    echo "Host IPv4 addresses:"
    list_host_ipv4_addresses | sed 's/^/  /'
    echo

    echo "Listeners on :{{port}}"
    lsof -nP -iTCP:{{port}} -sTCP:LISTEN || true
    echo

    echo "Backend listener on :8080"
    lsof -nP -iTCP:8080 -sTCP:LISTEN || true
    echo

    echo "Local HTTPS status"
    curl -4 -k -sS --connect-timeout 3 -D - "https://127.0.0.1:{{port}}/api/pi/v1/status" -o /tmp/tcube-local-status.json || true
    echo

    if [ -n "$lan_address" ]; then
        echo "LAN HTTPS status"
        curl -4 -k -sS --connect-timeout 3 -D - "https://$lan_address:{{port}}/api/pi/v1/status" -o /tmp/tcube-lan-status.json || true
    fi

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
