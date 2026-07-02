# Contributing

This guide explains how to work on `tcube-pi` without drifting from the T-Cube product constraints.

## Project References

Read these first:

* [VISION.md](VISION.md): Product contract, user experience, AI boundaries, and hardware concept.
* [docs/developer/current-state.md](docs/developer/current-state.md): Current implementation status and known risks.
* [docs/tasks.md](docs/tasks.md): Active and upcoming work.

Then route by task:

* [docs/developer/architecture-guide.md](docs/developer/architecture-guide.md): Runtime split, data boundaries, API boundaries, and deployment architecture.
* [docs/developer/rust-guide.md](docs/developer/rust-guide.md): Rust implementation rules.
* [docs/developer/testing-guide.md](docs/developer/testing-guide.md): Validation workflow.
* [docs/developer/branding-guide.md](docs/developer/branding-guide.md): Admin UI visual design and copy.
* [docs/hardware/hardware-assembly.md](docs/hardware/hardware-assembly.md): Hardware inventory, wiring, and bring-up.
* [docs/hardware/pi-os-lite-install.md](docs/hardware/pi-os-lite-install.md): Fresh Pi OS Lite and release-bundle install.

The core product rule is that child-facing button feedback must be immediate, deterministic, local, and never blocked by AI, network, dashboard, or reporting work.

## Development Stack

Current stack:

* Rust for the child-facing device runtime and Pi-hosted admin API.
* SQLite for local durable setup, content, and event storage.
* Caddy for the local HTTPS browser boundary.
* `just` for documented project commands.

Development runs directly on the host. The first Raspberry Pi runtime path stays native so GPIO, audio, and systemd behavior are validated without a container boundary.

## macOS Setup

Install local tools:

```sh
brew install rust just sqlite caddy
```

Validate the repo:

```sh
just check
just test
```

If `just` is not installed yet, run the underlying commands:

```sh
env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo check --all-features
env -u CFLAGS -u CXXFLAGS -u CPPFLAGS -u LDFLAGS cargo test --all-features
```

The `env -u ...` wrapper prevents host C/C++ flags from leaking into native Rust dependencies such as bundled SQLite.

## Local Button Simulator

Run the simulator:

```sh
just run-device-sim
```

Run the simulator with local audio playback:

```sh
just run-device-sim-audio
```

Controls:

* `1`: English
* `2`: Animals
* `3`: Music
* `4`: Setup/help during first-run
* `5`: Setup/help during first-run
* `q`: Quit

Inspect recent events:

```sh
sqlite3 data/tcube.sqlite3 'select occurred_at, button_id, mode, response_id from button_events order by id desc limit 10;'
```

Before setup is complete, button presses are written to `setup_debug_events` instead of normal usage logs.

## Pi Admin Service

Run the Pi-hosted Rust admin API:

```sh
just run-pi-admin
```

Use [the Caddy deployment guide](deploy/pi-admin-caddy/README.md) for the supported HTTPS boundary.

## Raspberry Pi Setup

Use Raspberry Pi OS Lite 64-bit on Raspberry Pi Zero 2 W.

Track fresh Pi setup, required packages, and release-bundle installation in [docs/hardware/pi-os-lite-install.md](docs/hardware/pi-os-lite-install.md). For the current baseline:

```sh
sudo apt update
sudo apt install -y ca-certificates caddy curl git just sqlite3 build-essential pkg-config
```

Install Rust with rustup unless the project later pins another Pi-specific install path:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Run native validation on the Pi:

```sh
just check
just test
```

## Workflow Rules

Keep changes small and scoped to the requested behavior.

Use `dev` as the default development branch for day-to-day work and pull requests. Keep `main` as the stable branch for release-ready promotion; merge `dev` into `main` when preparing stable releases or install-doc-visible changes.

Prefer established project patterns over new abstractions. Reuse the existing Rust runtime split, `just` recipes, SQLite event path, and docs structure before adding alternatives.

Do not introduce Makefiles or duplicate command runners. Add or update `Justfile` recipes when a workflow becomes reusable.

Do not commit secrets, `.env` files, credentials, local databases, or sensitive audio/data artifacts.

Do not commit generated build output. `target/`, local SQLite databases, local media, and `admin-ui/` are ignored.

When adding, removing, replacing, or seriously considering physical hardware, update [docs/hardware/hardware-assembly.md](docs/hardware/hardware-assembly.md) in the same change.

Use conventional commit messages if a commit is requested, such as `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, or `chore:`.

Do not include agent branding, assistant names, or co-authorship metadata in commits unless explicitly requested.

## Testing Expectations

Run targeted validation before handoff:

```sh
just test
```

For broader validation:

```sh
just check
just test
```

Add or update tests for business logic changes, especially button mapping, response selection, event logging, setup state transitions, content validation, and future GPIO/audio/LED backends.

If full validation cannot be run, document what was skipped and why.

## Documentation Expectations

Update docs when changing behavior, architecture, setup, hardware assumptions, or workflows.

Use single-line Markdown paragraphs and concise headings.

Keep volatile status in [docs/developer/current-state.md](docs/developer/current-state.md), not in `AGENTS.md`.
