# Testing Guide

This guide defines the maintained validation path for the Rust device runtime and Pi-hosted admin service with Caddy.

## Required Gates

Run targeted tests while developing, then run both repository gates before handoff:

```sh
just check
just test
```

`just check` verifies Rust formatting, compilation, and Clippy with warnings denied. `just test` runs the Rust test suite.

For admin UI source changes, run:

```sh
just build-admin-ui
just check-admin-ui
```

`just build-admin-ui` emits static assets into `admin-ui/build/`, which is the directory served by `tcube-pi-admin` and copied into Pi packages. `just check-admin-ui` runs Svelte and TypeScript checks against `admin-ui/`.

Use `pnpm` for every direct admin UI and JavaScript command. Prefer the `just` recipes above for normal workflows.

For Rust changes, follow the [Rust Guide](rust-guide.md), `rustfmt.toml`, `clippy.toml`, `.cargo/config.toml`, and `rust-toolchain.toml`. Use `just fmt`, `just fmt-check`, and `just lint` for focused iteration.

Host C/C++ flags are forced empty by `.cargo/config.toml` because inherited local flags can break native dependencies such as bundled SQLite.

GitHub Actions CI runs the Rust formatting, check, Clippy, and test gates plus the admin UI pnpm install, Svelte check, and static build gates. The release workflow repeats those gates, builds Linux arm64 release binaries on an arm64 runner, and publishes an installable Pi Zero 2 W bundle for version tags and manual dispatches.

Before creating a release tag, run `just prepare-release vX.Y.Z`, commit the manifest updates, then create the tag. The release workflow verifies that the tag version matches `Cargo.toml` and `admin-ui/package.json`.

## Device Runtime

Run the keyboard simulator without or with local audio:

```sh
just run-device-sim
just run-device-sim-audio
```

Press `1` through `5` for the cube faces and `q` or `Esc` to exit. Confirm the selected response appears immediately and the event is logged without waiting for network or AI work.

Inspect recent runtime events when needed:

```sh
sqlite3 data/tcube.sqlite3 'select occurred_at, button_id, mode, response_id from button_events order by id desc limit 10;'
```

Before setup completion, inspect `setup_debug_events` instead. Physical GPIO and audio validation must run natively on the Pi with `just run-device-pi`; do not treat simulator success as hardware validation.

## Pi Admin And Caddy

Start the loopback-only Rust admin service and validate the Caddy configuration:

```sh
just run-pi-admin
just validate-pi-admin-caddy
```

Run `tcube-pi-admin` and Caddy in separate terminals. Start Caddy with:

```sh
caddy run --config deploy/pi-admin-caddy/Caddyfile
```

Verify the direct backend and HTTPS boundary:

```sh
curl http://127.0.0.1:8080/api/pi/v1/status
curl -k https://localhost/api/pi/v1/status
```

`curl -k` is acceptable only for a local command-line smoke test. Browser and phone validation must trust Caddy's internal root CA and must exercise login, setup, content activation, upload, recording, generated speech, and logout through HTTPS.

Measure admin-load impact while `tcube-pi-admin` is running:

```sh
just measure-pi-admin
```

Installation, CA trust, service files, URLs, and the complete browser checklist live in [Pi Admin HTTPS With Caddy](../../deploy/pi-admin-caddy/README.md). Pi packages are tracked in [Raspberry Pi Package List](pi-package-list.md).

## External Speech Generation

`tcube-pi-admin` can request generated speech drafts from external HTTPS speech services. Those workers are intentionally outside this repository.

When changing speech-provider integration, verify the configured provider health and generate one short phrase through the admin API. Confirm that generated files are non-empty, playable, intelligible, and remain inactive until an administrator reviews and activates them on the cube.

## Maintenance Matrix

* Changes under `src/`: run `just fmt-check`, `just lint`, `just test`, then the relevant simulator or Pi smoke test.
* Changes under `admin-ui/`: run `just build-admin-ui` and `just check-admin-ui`, then smoke the Rust-served `/` route when practical.
* Changes under `deploy/pi-admin-caddy/`: run `just validate-pi-admin-caddy` and both direct and HTTPS status checks.
* Cross-boundary changes: run `just check`, `just test`, and every affected smoke path above.
