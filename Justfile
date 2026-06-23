set dotenv-load := true

default:
    just --list

dev-shell:
    docker compose run --rm dev

check:
    cargo fmt --all --check
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo check --all-features
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo clippy --all-targets --all-features -- -D warnings

build:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo build --workspace --all-features

build-release:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo build --workspace --all-features --release

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo clippy --all-targets --all-features -- -D warnings

test:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo test --all-features

run-device-sim:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo run --bin tcube-pi -- --backend sim

run-device-sim-audio:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo run --bin tcube-pi -- --backend sim --audio local

run-device-pi:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo run --bin tcube-pi -- --backend pi

run-pi-admin:
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo run --bin tcube-pi-admin -- --bind 127.0.0.1:8080 --database data/tcube.sqlite3 --ui-dist admin-ui/build --media-root data/media --content-root content --hostname tcube.local --usb-address 10.55.0.1

install-admin-ui:
    pnpm --dir admin-ui install

build-admin-ui:
    pnpm --dir admin-ui run build

check-admin-ui:
    pnpm --dir admin-ui run check

validate-pi-admin-caddy:
    caddy validate --config deploy/pi-admin-caddy/Caddyfile

measure-pi-admin button_presses='1000' admin_requests='600' admin_workers='4':
    env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo run --bin tcube-pi-admin-measure -- --base-url http://127.0.0.1:8080 --content content/content.json --button-presses {{button_presses}} --admin-requests {{admin_requests}} --admin-workers {{admin_workers}}
