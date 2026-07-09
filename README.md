# tcube-pi

`tcube-pi` is the Raspberry Pi Zero 2 W runtime and local parent-admin service for T-Cube.

The repo owns child-facing button/audio behavior, local SQLite state, approved content playback, the Pi-hosted admin API, checked-in static admin UI output, Caddy/systemd deployment files, and Pi release-bundle packaging.

It does not own cloud services, Mac speech workers, hosted sync, or child-facing screen UI.

## Agent Entry

Coding agents should start with [AGENTS.md](AGENTS.md). It is the compact routing contract for default context, task-specific docs, validation gates, and repo-specific safety boundaries.

Use [VISION.md](VISION.md) for product constraints, [docs/tasks.md](docs/tasks.md) for active priorities, and [docs/developer/current-state.md](docs/developer/current-state.md) for the live implementation snapshot.

Use [docs/developer/README.md](docs/developer/README.md) to choose the focused developer doc for Rust, architecture, testing, branding, hardware, or install work.

## Quick Start

Install local development tools on macOS:

```bash
brew install rust just sqlite caddy
```

Run the Rust validation gate:

```bash
just check
just test
```

Run the keyboard simulator:

```bash
just run-device-sim
just run-device-sim-audio
```

Run the Pi-hosted admin service locally:

```bash
just run-pi-admin
```

In a second terminal, start local HTTPS for browser and phone testing:

```bash
just run-pi-admin-lan-caddy
```

Open `https://127.0.0.1:8443/` on the same machine or the printed `https://<lan-ip>:8443/` URL from a phone on the same network.

## Repository Map

- [src/](src/): Rust runtime, Pi admin service, and admin-load measurement harness.
- [content/](content/): default approved local content and audio assets.
- [admin-ui/](admin-ui/): Svelte + Vite source for the parent dashboard.
- [admin-ui/build/](admin-ui/build/): checked-in static dashboard build served by `tcube-pi-admin`.
- [deploy/pi-admin-caddy/](deploy/pi-admin-caddy/): Caddy and systemd files for the installed Pi admin service.
- [deploy/pi-runtime/](deploy/pi-runtime/): systemd and env files for the installed child-facing runtime.
- [deploy/pi-release/](deploy/pi-release/): release-bundle preparation and Pi installer scripts.
- [deploy/pi-zero-button-smoke/](deploy/pi-zero-button-smoke/): temporary one-button bench smoke payload.
- [docs/developer/](docs/developer/): routed architecture, testing, Rust, branding, and state docs.
- [docs/hardware/](docs/hardware/): hardware assembly and Raspberry Pi OS Lite install docs.

## Binaries

- `tcube-pi`: child-facing runtime and keyboard simulator.
- `tcube-pi-admin`: local admin API and static dashboard server.
- `tcube-pi-admin-measure`: latency harness for admin-load impact.

## Admin UI

Use Bun for every admin UI workflow. Prefer the `just` recipes unless you are working directly inside `admin-ui/`.

```bash
just install-admin-ui
just build-admin-ui
just check-admin-ui
just test-admin-ui-unit
just test-admin-ui-mobile
```

The Pi does not run Node or Bun. Release packaging copies built files from `admin-ui/build/`, so rebuild that directory after UI source changes.

## Pi Install And Release

Fresh Raspberry Pi OS Lite setup lives in [docs/hardware/pi-os-lite-install.md](docs/hardware/pi-os-lite-install.md).

Installed HTTPS, Caddy CA trust, and browser access details live in [deploy/pi-admin-caddy/README.md](deploy/pi-admin-caddy/README.md).

Release-bundle preparation and installer behavior live in [deploy/pi-release/README.md](deploy/pi-release/README.md).
