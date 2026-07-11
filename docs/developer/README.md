# Developer Documentation Index

Read `AGENTS.md`, `VISION.md`, `docs/tasks.md`, and `current-state.md` first. Then load only the document that matches the work.

## Core Routes

- [Current State](current-state.md): live implementation snapshot, known gaps, durable decisions, and latest validation.
- [Architecture Guide](architecture-guide.md): runtime/admin split, API boundaries, data flow, storage, sync, privacy, and deployment architecture.
- [Rust Guide](rust-guide.md): Rust module boundaries, async/database/hardware rules, error policy, and Rust review triggers.
- [Testing Guide](testing-guide.md): validation commands, simulator checks, Pi admin/Caddy smoke tests, and handoff expectations.
- [Branding Guide](branding-guide.md): admin UI visual design, copy, status colors, component direction, and accessibility expectations.
- [Orchestration Blueprint](orchestration-blueprint.md): proposed Mac Mini Hermes Agent pipeline, wire contract, Mac stack under `deploy/mac-hermes/`, and the Pi-side Rust task plan.

## Related Routes

- [Project Vision](../../VISION.md): product contract and non-goals.
- [Task List](../tasks.md): active priority backlog.
- [Hardware Assembly](../hardware/hardware-assembly.md): hardware inventory, wiring, and bring-up order.
- [Raspberry Pi OS Lite Install](../hardware/pi-os-lite-install.md): fresh OS setup, release-bundle install, package list, and service checks.
