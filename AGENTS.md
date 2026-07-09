# Agent Instructions

Repo-specific router for `tcube-pi`. Global agent rules cover generic coding style, edit safety, commits, Markdown style, and default package-manager preferences.

## Purpose

`tcube-pi` owns the Raspberry Pi Zero 2 W runtime for T-Cube: child-facing button/audio behavior, local SQLite state, the Pi-hosted parent admin API, checked-in static admin UI output, Caddy/systemd deployment files, and release-bundle packaging.

## Start Here

Read only this default context first:

- `docs/tasks.md` for active priorities.
- `VISION.md` for product constraints.
- `docs/developer/README.md` for doc routing.
- `docs/developer/current-state.md` for live implementation state.
- `git status --short` for the current worktree.

Then inspect only the files and routed docs relevant to the requested change. If a referenced doc is missing, continue and note it.

## Routes

- Rust implementation: `docs/developer/rust-guide.md`.
- Architecture, API boundaries, runtime/admin split, storage, sync, or privacy: `docs/developer/architecture-guide.md`.
- Validation, simulator checks, Pi admin/Caddy smoke tests, or handoff planning: `docs/developer/testing-guide.md`.
- Admin UI visual design, copy, layout, or component styling: `docs/developer/branding-guide.md`.
- Hardware parts, wiring, physical bring-up, or component changes: `docs/hardware/hardware-assembly.md`.
- Fresh Raspberry Pi OS Lite install or release-bundle setup: `docs/hardware/pi-os-lite-install.md`.
- Release-bundle scripts and installer behavior: `deploy/pi-release/README.md`.

## Workflows

Use `just` as the repo workflow surface; do not introduce Makefiles or ad hoc parallel command catalogs.

Rust gate before handoff: `just check` and `just test`.

Admin UI gate before handoff: `just build-admin-ui`, `just check-admin-ui`, `just test-admin-ui-unit`, and `just test-admin-ui-mobile`.

Admin UI source under `admin-ui/` intentionally uses Bun. Use the `just` recipes above for normal work; if running direct JavaScript commands, use `bun` with `admin-ui/bun.lock`.

`admin-ui/build/` is checked-in static output served by `tcube-pi-admin` and copied into Pi release bundles, so rebuild it after UI source changes.

For local browser or phone testing, run the loopback Rust service with `just run-pi-admin` and the HTTPS front door with `just run-pi-admin-lan-caddy`.

Physical GPIO, I2S audio, installed services, boot behavior, and Pi resource validation must run on target hardware; simulator success is not hardware validation.

Before release tagging, run `just prepare-release vX.Y.Z`, review the manifest updates, then validate. Release tags must point at commits reachable from `main`, and manual release workflow runs must start from `main`.

## Boundaries

Keep `AGENTS.md` as a short router, not a session log or architecture manual. Put durable implementation state in `docs/developer/current-state.md` and detailed workflows in the routed docs.

Never commit secrets, `.env` files, local SQLite databases, or sensitive generated/recorded audio under `data/`.

When adding, removing, replacing, or seriously considering a physical component, update `docs/hardware/hardware-assembly.md`.

Agent-specific adapter files such as `CLAUDE.md` or `COPILOT.md` should stay thin and reference this file instead of duplicating repo-wide rules.
