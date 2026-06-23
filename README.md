# tcube-pi

`tcube-pi` is the Raspberry Pi runtime for T-Cube.

It owns the child-facing Rust runtime, local SQLite state, cached content playback, the Pi-hosted admin API, and the Caddy-backed local HTTPS deployment path.

## Install

Install local development tools on macOS:

```bash
brew install rust just sqlite caddy
just check
just test
```

Run the TUI keyboard simulator:

```bash
just run-device-sim
just run-device-sim-audio
```

Run the Pi-hosted admin service:

```bash
just run-pi-admin
```

Use Caddy for HTTPS browser access by following [the Pi admin Caddy guide](deploy/pi-admin-caddy/README.md).

## Repository Map

- [src/](src/): Rust runtime, Pi admin service, sync client/API, and measurement harness
- [content/](content/): default approved local content and audio assets
- [admin-ui/](admin-ui/): Svelte + Vite source for the parent/admin dashboard
- [admin-ui/build/](admin-ui/build/): checked-in static admin UI build output served by `tcube-pi-admin`
- [deploy/pi-admin-caddy/](deploy/pi-admin-caddy/): Caddy and systemd deployment files for the Pi admin service
- [deploy/pi-zero-button-smoke/](deploy/pi-zero-button-smoke/): temporary one-button bench smoke payload
- [docs/developer/](docs/developer/): architecture, testing, Rust, auth, package, and state docs
- [docs/hardware/inventory.md](docs/hardware/inventory.md): tracked hardware components

## Binaries

- `tcube-pi`: child-facing runtime and simulator
- `tcube-pi-admin`: Pi-hosted admin API
- `tcube-pi-admin-measure`: admin-load latency harness
- `tcube-pi-device-api`: device content sync API compatibility harness

## Validation

```bash
just check
just test
```

For Rust changes, keep `cargo fmt`, Clippy with warnings denied, and tests clean before handoff.

Build and type-check the static admin UI:

```bash
just install-admin-ui
just build-admin-ui
just check-admin-ui
```

The Pi does not run Node. Release packaging copies the built files from `admin-ui/build/`, so rebuild that directory after UI source changes.

Use `pnpm` for every admin UI and JavaScript workflow. Do not use npm, yarn, bun, or ad hoc global JavaScript tooling for this UI.

## Project Context

Read [VISION.md](VISION.md), [docs/tasks.md](docs/tasks.md), and [docs/developer/current-state.md](docs/developer/current-state.md) for the current product contract and implementation status.
