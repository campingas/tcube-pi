# Rust Guide

This guide applies to Rust files in this workspace. Inspect the current `src/` tree before editing; do not rely on stale file-ownership memory.

## Quality Gates

Run these before handoff for Rust changes:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The repository pins stable Rust in `rust-toolchain.toml` and normalizes inherited host C/C++ flags in `.cargo/config.toml`.

## Core Rules

- Keep changes scoped to the requested Rust behavior.
- Keep binaries thin; parse config/flags in entry points and delegate reusable behavior to library modules.
- Prefer simple, explicit code over clever abstractions.
- Prefer pure helpers for selection, debounce, validation, metrics, and formatting logic.
- Keep side effects at system boundaries: hardware, filesystem, database, network, and process startup.
- Add tests for behavioral changes.
- Do not normalize user experiments or unrelated code.
- Do not modify Git/Jujutsu history.

## Module Boundaries

Use the current tree as source of truth. These durable boundaries matter more than file names:

- Config loading belongs near config modules, not in binaries or handlers.
- Domain/event types should be defined once and shared.
- Hardware access belongs behind hardware/runtime abstractions and must stay simulator-safe.
- Admin transport belongs in Axum route/server modules.
- SQLite helpers should execute storage operations and return typed rows/domain structs; avoid product decisions in storage helpers.
- Metrics and latency helpers must not touch hardware or network.
- Static admin UI/media serving must keep path traversal rejection and correct response metadata.

When a module mixes unrelated responsibilities, split by ownership: config, domain types, hardware, server transport, storage, metrics, media, speech, and binary wiring.

## Axum And Async

- Assemble the production router in the server entry layer; expose focused sub-routes or handlers from route modules.
- Prefer typed extractors (`State`, `Json`, `Path`, `Query`, multipart, cookies) over manual request parsing.
- Handlers should return response types through the established route error pattern.
- Do not block inside async functions. Use `tokio::task::spawn_blocking` for CPU-bound or blocking I/O.
- Keep async functions focused on coordination and push computation into sync helpers.
- Do not hold any mutex guard across `.await`.
- Bound network and external-provider work with timeouts.
- Avoid fire-and-forget tasks without shutdown or error visibility.

## SQLite Rules

- `rusqlite::Connection` is not `Send`; do not move it into async tasks or hold it across `.await`.
- Dispatch SQLite work through the established blocking helper or `spawn_blocking`.
- Run schema setup/migrations before serving requests.
- Use WAL mode for SQLite files shared by runtime/admin/measurement binaries.
- Keep query helpers explicit about ordering, filtering, and range behavior.
- Add indexes with new high-volume query paths.
- Use in-memory SQLite only for tests.

## Hardware Rules

- GPIO, I2S, LED, IMU, and microphone work must be isolated from admin/server modules.
- Hardware initialization failures are startup failures.
- Long-running hardware loops should run on dedicated threads or bounded channels, not unbounded async fire-and-forget tasks.
- Do not busy-poll hardware in tight loops.
- Gate hardware-only code so desktop `cargo test` works without Pi devices.
- Simulator behavior should remain available for development without target hardware.

## Error And Panic Policy

- Runtime fallible paths return `Result` with useful context.
- Avoid silent recovery that hides root cause.
- Do not introduce production `panic!`, `unwrap`, or `expect` unless the invariant is impossible by design and explained.
- `unwrap`/`expect` are acceptable in tests when they keep the test clear.
- Keep comments purposeful: why, assumptions, constraints, and safety.
- Remove stale comments.

## Testing Rules

- Keep one behavior per test with descriptive names.
- Cover success and error paths for changed behavior.
- Prefer unit tests for pure helpers and storage helpers; prefer route/service tests for public HTTP behavior.
- Database tests should use isolated temporary or in-memory databases.
- Hardware behavior should have simulator-safe tests unless the validation explicitly requires target hardware.
- Do not spin up real TCP listeners for unit tests when Axum/Tower service tests can exercise the route.

## High-Risk Review Triggers

- Any code added to the button response path.
- Any `.await` while holding a lock.
- Any SQLite access outside the blocking pattern.
- Any hardware call outside runtime/hardware boundaries.
- Any SQL string added outside storage modules.
- Any content activation change that could expose drafts or corrupt media to the runtime.
- Any new external network dependency.
- Missing error context, untested parsing, unchecked slice reads, or unbounded file uploads.
- Unnecessary cloning or allocation in hot paths.
