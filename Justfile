set dotenv-load := true

default:
    just --list

dev-shell:
    docker compose run --rm dev

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
    cargo run --bin tcube-pi-admin -- --bind 127.0.0.1:8080 --database data/tcube.sqlite3 --ui-dist admin-ui/build --media-root data/media --content-root content --hostname tcube.local --usb-address 10.55.0.1

install-admin-ui:
    pnpm --dir admin-ui install

build-admin-ui:
    pnpm --dir admin-ui run build

check-admin-ui:
    pnpm --dir admin-ui run check

validate-pi-admin-caddy:
    caddy validate --config deploy/pi-admin-caddy/Caddyfile

measure-pi-admin button_presses='1000' admin_requests='600' admin_workers='4':
    cargo run --bin tcube-pi-admin-measure -- --base-url http://127.0.0.1:8080 --content content/content.json --button-presses {{button_presses}} --admin-requests {{admin_requests}} --admin-workers {{admin_workers}}
