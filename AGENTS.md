# Agent Instructions

Keep this project agent-agnostic. This file defines the stable repo-wide contract for coding agents and contributors.

## Priority Order

When instructions conflict, follow this order:

1. Direct user request
2. This `AGENTS.md`
3. Product direction in `VISION.md`
4. Routed repo docs under `docs/`
5. Existing codebase patterns
6. Agent-specific adapter files such as `CLAUDE.md` or `COPILOT.md`

## Session Start

Read only the default context first:

- `docs/tasks.md` for active priorities
- `VISION.md` for product constraints
- `docs/developer/README.md` for doc routing
- `docs/developer/current-state.md` for the live implementation snapshot
- `git status --short`

Then inspect only the files and routed docs relevant to the requested change. If a referenced file is missing, continue and note it rather than failing.

## Doc Routing

- Rust implementation: read `docs/developer/rust-guide.md`.
- Architecture, API boundaries, runtime/admin split, storage, sync, or privacy behavior: read `docs/developer/architecture-guide.md`.
- Validation or handoff planning: read `docs/developer/testing-guide.md`.
- Admin UI visual design, copy, layout, or component styling: read `docs/developer/branding-guide.md`.
- Hardware parts, wiring, or physical bring-up: read `docs/hardware/hardware-assembly.md`.
- Fresh Raspberry Pi OS Lite install or release-bundle setup: read `docs/hardware/pi-os-lite-install.md`.

## Context Maintenance

Do not use `AGENTS.md` as a session log.

At the end of significant work, update `docs/developer/current-state.md` only with live state that future agents need: current status, durable decisions, important assumptions, known issues, pending TODOs, and recommended next steps. Do not append long chronological history.

## Core Working Rules

- Read directly related files before editing; do not infer behavior from filenames alone.
- When changing behavior, inspect relevant tests, config, adjacent modules, and routed documentation.
- Prefer existing codebase patterns and utilities over new abstractions.
- Keep changes scoped to the requested behavior; note unrelated issues separately.
- Do not revert user changes unless explicitly requested.
- Do not commit unless explicitly asked.
- Use conventional commit messages if a commit is requested.
- Never include agent branding, assistant names, or co-authorship metadata unless explicitly requested.

## Environment And Safety

- Do not start long-running dev servers, watchers, or background processes unless explicitly requested.
- Prefer one-shot validation commands.
- On macOS, use `/usr/bin/open` instead of plain `open`.
- Never commit secrets, credentials, `.env` files, config secrets, local databases, or sensitive audio/data artifacts.
- Treat local data stores as sensitive unless clearly intended for version control.
- Prefer safe, explicit deletion commands and never delete files you have not inspected unless explicitly instructed.

## Tooling Conventions

- This repository is Rust-first.
- Use `just` and the `Justfile` for documented project workflows; do not introduce Make or a `Makefile`.
- The admin UI under `admin-ui/` is an intentional repository-owned JavaScript package and uses `bun`; do not use npm, yarn, pnpm, or ad hoc global JavaScript tooling for it.
- Use `uv` for Python environments and `uvx` for one-off Python CLI tools when needed.
- Match existing formatter and lint rules.
- Use lowercase kebab-case filenames unless the surrounding area uses a different convention.
- Executable scripts should not use file extensions unless the ecosystem or existing repo conventions require them.

## Rust Changes

For Rust changes, follow `docs/developer/rust-guide.md`, `rustfmt.toml`, and `clippy.toml`.

Before handoff for Rust changes, run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Treat rustc and Clippy diagnostics as actionable.

## Validation

Run targeted tests first, then repo-level validation where appropriate. If full validation cannot be run, state what was and was not verified.

For admin UI changes, prefer the documented `just` recipes in `docs/developer/testing-guide.md`.

## Documentation

- Use single-line paragraphs in Markdown.
- Keep headings concise.
- Update routed docs when behavior, architecture, setup, hardware assumptions, or workflows change.
- When adding, removing, replacing, or seriously considering any physical component, device, module, or major material, update `docs/hardware/hardware-assembly.md`.

## Agent Adapters

Agent-specific instructions should live in thin adapter files that reference this file instead of duplicating repo-wide rules.
