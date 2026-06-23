# Rust Guide

This guide applies to all Rust files matching `**/*.rs`.

Correctness, intent clarity, and maintainability come first; performance changes should be based on evidence.

## Core Rules

- Never clean up, replace, or normalize in-progress user experiments unless explicitly asked.
- Never modify Git or Jujutsu history.
- Keep Rust changes scoped to the requested Rust workspace area.
- Favor simplicity. Write the least complex code that solves the problem; clarity beats cleverness.
- Prefer simple code over elegant code, and prefer elegant code over complex code.
- Prefer a functional-first Rust style: immutability, pure helpers, and composition over shared mutable state.
- When the user asks for an explicit code change, do not add comments unless they are needed to explain safety, constraints, or non-obvious intent.
- Keep transform chains readable and line-broken.
- Always verify `jj status` before starting work; if unavailable, use `git status --short --branch`.
- Before merge or commit decisions, review the current branch for behavior regressions, performance risks, and test coverage gaps.
- Avoid magic strings. Extract string literals used as identifiers or keys into named constants at the top of the file or in a dedicated constants module.
- Prefer explicitness over magic.

## Quality Gates

Minimum Rust quality gates for any Rust change:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The repository pins the stable Rust toolchain in `rust-toolchain.toml` and forces host C/C++ flags empty in `.cargo/config.toml` so native dependencies build consistently across developer machines.

Required pre-merge quality gates:

- Intentional ownership and data-flow review.
- Explicit and aligned error handling.
- `cargo fmt --all --check`.
- `cargo clippy --all-targets --all-features --locked -- -D warnings`.
- `cargo test --workspace --all-features`.
- Measured performance claims before acceptance.
- Documentation reflects runtime behavior.

## `tcube-pi` Module Boundaries

The Rust device crate is library-backed. Keep `src/main.rs` and every file in `src/bin/` as thin entry points that only parse flags, construct or load config, call one library function, and return or print top-level errors.

Current module ownership:

- `src/config.rs` owns process configuration structs and config loading helpers.
- `src/error.rs` owns shared typed errors and the shared `Result` alias for library boundaries.
- `src/events/types.rs` owns shared event, content, button, impact, and measurement data types used across binaries.
- `src/hardware/gpio.rs` owns GPIO initialization and pin abstractions.
- `src/hardware/button.rs` owns button input, debounce/state-machine behavior, simulator input, runtime loop wiring, local audio/LED boundary calls, and button-response event logging.
- `src/server/mod.rs` owns the Pi admin Axum/Tokio router entry point exposed as `server::run(config)`.
- `src/server/handler.rs` owns HTTP request adaptation, route dispatch, and auth/setup/content/media mutations.
- `src/server/pages.rs` owns built admin UI serving, static media/content file responses, MIME selection, traversal checks, and HTML page rendering functions when server-rendered pages are needed.
- `src/db/measurements.rs` owns measurement storage operations only: schema migration, inserts, and queries.
- `src/metrics/latency.rs` owns latency measurement, rolling/statistical math, spike detection, and admin-load measurement logic.

Do not add business logic to `db/` modules. Database modules should take explicit inputs, execute storage operations, and return typed rows or domain structs without deciding product behavior.

Do not add request-routing or database mutation behavior to `server/pages.rs`. Page modules should only turn paths and files into HTTP responses; route dispatch stays in `server/handler.rs`.

Do not add hardware access to metrics modules. Metrics helpers should stay testable without GPIO, audio devices, network listeners, or physical Pi state.

## Future Expansion Practices

When adding a new binary, add a `[[bin]]` entry in root `Cargo.toml`, keep the entry point thin, and place reusable behavior under `src/`.

When adding a new device subsystem, create a focused module under the closest existing boundary before adding a new top-level module. For example, add LED driver code under `hardware/` and synchronization-client code under a new module only after its ownership is clear.

When adding shared structs, define them once in `events/types.rs` or a dedicated domain module and import them from all binaries. Avoid duplicate request, event, measurement, or content-pack structs in binary files.

When adding latency, debounce, selection, scheduling, or spike-detection logic, prefer pure functions with unit tests. Keep I/O at the outer layer and pass measured values or snapshots into pure helpers.

When expanding the Pi admin server, keep the Axum router in `server::run`, route matching in `server::handler`, storage helpers in the relevant `db/` module, and rendering in `server::pages`.

When adding SQLite schema, expose an idempotent `run_migrations(conn: &Connection) -> Result<()>` function in the owning `db/` module and keep query helpers explicit about their range, ordering, and filtering behavior.

When a module starts mixing unrelated responsibilities, split by ownership rather than by file size. The preferred split is boundary-based: config, domain types, hardware, server transport, storage, metrics, and binary wiring.

## Axum Rules

- The Axum router is assembled once in `server::run`. Route registration does not belong in handler modules; handlers expose functions or `fn router() -> Router` sub-routers that are merged at the top level.
- Handler functions must be `async fn` returning `impl IntoResponse`. Do not return `anyhow::Result` directly from a handler — use the `AppError` wrapper defined in `error.rs`.
- Define `AppError` once in `error.rs` and implement `IntoResponse` for it there. Do not define per-handler error types.

```rust
// error.rs
#[derive(Debug)]
pub struct AppError(pub anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self { AppError(e.into()) }
}
```

- Use `State(state): State<AppState>` for dependency injection. Do not use globals, `lazy_static`, or `once_cell` to hold runtime state accessible from handlers.
- `AppState` must be `Clone`. Wrap non-`Clone` resources in `Arc`. Wrap non-`Send` resources in `Arc<Mutex<T>>`.
- Keep `AppState` flat and minimal. Do not put business logic or config derivation inside state types.
- Prefer typed extractors (`Json<T>`, `Path<T>`, `Query<T>`) over manual request parsing. Validate at the extractor boundary, not deep in handler logic.
- Use `axum::middleware::from_fn` or Tower layers for cross-cutting concerns (auth, logging, timeouts). Do not inline auth checks into individual handlers.
- Do not use `axum::response::Redirect` for error cases — return an appropriate `StatusCode` and body instead.
- When serving static files, prefer `tower_http::services::ServeDir`. Do not hand-roll file serving in handlers.

## Tokio and Async Rules

- The Tokio runtime is entered once at `main.rs` via `#[tokio::main]`. Do not create nested runtimes with `Runtime::new()` or `block_on` inside async contexts.
- `rt-multi-thread` is the default feature. If binary size becomes a measured concern on the Pi Zero 2W, switch to `rt` (single-thread) and document the reason.
- Do not call blocking operations directly inside `async fn`. Blocking inside an async context starves the Tokio executor. Use `tokio::task::spawn_blocking` for CPU-bound or blocking I/O work.
- Keep `async` functions short and focused on coordination. Push computation into sync helpers called via `spawn_blocking` or tested pure functions.
- Prefer `tokio::sync::Mutex` over `std::sync::Mutex` only when the lock must be held across `.await` points. For short, non-yielding critical sections, `std::sync::Mutex` is preferred and has lower overhead.
- Do not hold any `MutexGuard` across an `.await` point — this is a deadlock risk and a compiler error with `std::sync::Mutex`.
- Use `tokio::time::timeout` to bound any network or I/O operation that could block indefinitely.
- Prefer `tokio::sync::broadcast` or `tokio::sync::watch` for event fan-out between tasks over shared `Vec` + `Mutex`.
- Avoid `tokio::task::spawn` for fire-and-forget work without a shutdown signal — use a `JoinHandle` or a `CancellationToken` to ensure clean teardown.

## rusqlite + Async Rules

- `rusqlite::Connection` is `!Send`. It must never be moved into an async task or held across an `.await` point.
- Wrap `Connection` in `Arc<std::sync::Mutex<Connection>>` in `AppState`. This is the correct pattern for the Pi admin server's single-writer workload.
- All rusqlite calls must be dispatched through `tokio::task::spawn_blocking`:

```rust
let conn = Arc::clone(&state.db);
let result = tokio::task::spawn_blocking(move || {
    let conn = conn.lock().expect("db mutex poisoned");
    db::measurements::insert_measurement(&conn, &measurement)
}).await??;
```

- Do not call `conn.lock()` and then `.await` before the guard is dropped. Acquire the lock, do the work, drop the guard — all within the `spawn_blocking` closure.
- Schema migrations (`run_migrations`) must run once at startup before the Axum router starts accepting connections, not lazily on first request.
- If multiple binaries write to the same SQLite file (e.g. `tcube-pi-admin-measure` and the server), enable WAL mode at connection open time:

```rust
conn.execute_batch("PRAGMA journal_mode=WAL;")?;
```

- Do not use SQLite in-memory databases for production paths — only in unit tests where isolation is required.
- Keep all query helpers in `db/measurements.rs`. Do not inline SQL strings into handler or metrics code.

## Hardware and Embedded Rules

- GPIO access belongs exclusively in `hardware/gpio.rs` and `hardware/button.rs`. No other module may import `rppal` or touch GPIO pins directly.
- Hardware initialization is blocking and must run before the Tokio runtime starts, or be dispatched via `spawn_blocking`. Do not initialize GPIO inside an `async fn` without `spawn_blocking`.
- The button event loop is a long-running blocking loop. Run it in a dedicated OS thread via `std::thread::spawn`, not inside a Tokio task. Communicate events to async code via `tokio::sync::mpsc`.
- Do not add `rppal` or hardware-specific dependencies to `dev-dependencies` — unit tests must compile and run on the development machine (macOS/Linux x86) without Pi hardware.
- Guard hardware-only code paths with `#[cfg(target_arch = "aarch64")]` or a `hardware` feature flag when necessary to keep `cargo test` working off-device.
- Never busy-poll GPIO state in a tight loop. Use interrupt-driven or debounce-delayed polling with a `std::thread::sleep` or `rppal` interrupt callback.
- Treat GPIO errors as fatal at startup. A device that cannot initialize its hardware should not proceed to serve requests.

## ratatui TUI Rules

- The TUI event loop runs in `src/main.rs` and the hardware simulator modules only. Do not import `ratatui` from admin service modules.
- Always restore the terminal on exit, including on panic. Use a guard struct that implements `Drop` to call `terminal::disable_raw_mode()` and `execute!(stdout, LeaveAlternateScreen)`.
- Do not block the TUI event loop on network or database I/O. Use `std::sync::mpsc` or `tokio::sync::mpsc` to receive data from background threads and poll with a timeout.
- Keep rendering logic in pure functions that take data and return nothing — side effects are limited to the `terminal.draw(|f| ...)` closure.
- Do not hold locks inside the `terminal.draw` closure.
- Use a fixed tick rate (e.g. 100ms `crossterm::event::poll` timeout) rather than blocking indefinitely on input events.

## Concurrency Between Binaries

- `tcube-pi-admin` (server), `tcube-pi-admin-measure`, and any future binaries may run concurrently against the same SQLite file. WAL mode is required (see rusqlite rules).
- Do not use file locks or advisory locks as a substitute for WAL — they are not portable and create silent failure modes.
- Shared state between binaries must go through SQLite or a clearly owned IPC channel. Do not use shared memory or unnamed pipes between binaries.
- Each binary opens its own `Connection`. Do not share a `Connection` across process boundaries.

## Feature Flags and Platform Targeting

- Use a `hardware` Cargo feature to gate `rppal` and GPIO-dependent code. This allows `cargo test` and `cargo check` to run on non-Pi machines without hardware stubs.
- Do not use `#[cfg(feature = "hardware")]` inside `db/`, `metrics/`, `events/`, or `server/` modules — those modules must compile cleanly on all platforms.
- Prototype-only code must be gated behind a `prototype` feature or removed before merge. Do not ship `cfg(debug_assertions)` workarounds for hardware behavior.
- Document non-obvious `cfg` gates with a comment explaining the target and why the gate is needed.

## Coding Rules

- Default to borrowing with `&T` or `&mut T`, and avoid cloning unless ownership is required.
- Prefer small `Copy` types by value; pass heap-backed or large types by reference.
- Use explicit ownership in public APIs; prefer `&str` and `&[T]` over owned inputs unless ownership is needed.
- Prefer `?` for fallible flow and use `let PATTERN = EXPR else { ... }` or `if let ... else` for structured early-exit patterns.
- Prefer `if` / `else if` / `else` over `if` with early return when it improves readability.
- Use `_or_else` and lazy defaults to avoid eager allocation when not needed.
- Prefer iterator pipelines for pure transformations; use `for` loops when `break`, `continue`, early return, or complex side effects are required.
- Keep import groups ordered as `std` / `core` / `alloc`, external crates, workspace crates, `super::`, then `crate::`.
- Prefer references before pointers; use `Arc` or `Rc` only when the ownership model requires it.
- Treat raw pointer and `unsafe` usage as bounded API surfaces with explicit invariants.
- Prefer `From` and `Into` trait conversions over manual bit-transform conversion implementations.
- Keep side effects at system boundaries such as I/O, engines, and FFI calls; keep transformation logic side-effect free.
- Use `Cell` or `RefCell` only when borrow rules must be relaxed.

## Errors and APIs

- Libraries should use typed errors such as `thiserror` with clear boundary conversions.
- Binaries may use `anyhow`, but must maintain informative context.
- Avoid silent recovery from `Err`; keep root-cause visibility where feasible.
- Public items should be documented with `///`, including behavior, parameters, returns, and error, panic, or safety notes.
- Keep comments purposeful: why, assumptions, constraints, and safety.
- Remove stale comments.
- Convert TODOs to issue-linked notes such as `// TODO(issue #NNN): ...`.
- Add module-level behavior notes with `//!` when needed.

## Panic and Unwrap Policy

- Restrict `unwrap` and `expect` to tests or impossible-by-design cases with rationale.
- Code must not introduce new `panic!`, `unwrap`, or `expect` unless explicitly approved.
- Runtime fallible paths must return `Result` with meaningful error context instead of aborting, unless explicitly required by the user.
- `unwrap` and `expect` are allowed in tests and short-lived prototype code only.
- `expect` messages must explain the invariant.
- Prototype-only panics and unwraps must be removed or isolated behind non-shipping `cfg` or feature gates before merge.
- The one permitted `expect` in production async code is on `Mutex::lock()` for a poisoned lock — document this explicitly with `// mutex poisoned: a panicking task left the db in an unknown state`.

## Testing and Review

- Keep one behavior per test, with descriptive test names.
- Cover success and error behavior for behavioral paths.
- Add integration tests for public behavior and unit tests for internals when appropriate.
- Use snapshot tests only for complex, stable structured outputs.
- Avoid unnecessary `#[allow]`; prefer explicit local `#[expect(...)]` only when justified.
- Metrics and latency helpers in `metrics/` must have unit tests that do not require hardware or a live database.
- Database helpers in `db/` must have unit tests using a temporary in-memory SQLite connection (`Connection::open_in_memory()`).
- Axum handler tests should use `axum::test` or `tower::ServiceExt::oneshot` — do not spin up a real TCP listener in unit tests.

## High-Risk Review Triggers

- Unnecessary cloning in hot paths.
- Unjustified `#[allow]` or suppression.
- Missing context on errors.
- Copying large values by value without justification.
- TODOs without ownership or context.
- Any `rusqlite::Connection` usage outside a `spawn_blocking` closure in async code.
- Any `.await` point while a `MutexGuard` is in scope.
- Any GPIO or hardware call outside `hardware/`.
- Any `ratatui` import outside the simulator/runtime path.
- Any SQL string outside `db/` modules.

## Dispatch Strategy

- Prefer static dispatch with `impl Trait` or generic bounds when types are known and performance matters.
- Use dynamic dispatch with `dyn Trait` for plugin-style or runtime polymorphism needs.
- Delay boxing until required by design boundaries.

## Type-State

- Use type-state patterns when illegal states can be prevented at compile time.
- Skip type-state when runtime checks are clearer for simple finite-state behavior.

## References

- Default review workflow references: Rust Best Practices standards and `.codex/skills/rust-code-review/agents/openai.yaml`.
