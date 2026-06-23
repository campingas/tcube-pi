# Architecture Guide

This guide defines the intended technical architecture for T-Cube.

The product constraint from `VISION.md` is the main architectural driver: child-facing button feedback must be immediate, deterministic, offline-first, and never blocked by AI, dashboard, network, or reporting work.

## Preferred Stack

Hardware:

* Raspberry Pi Zero 2 W with pre-soldered GPIO headers
* GPIO button input
* Local speaker, LEDs, microphone, and battery hardware
* Local Mac mini for LLM content generation

Software:

* Rust for the child-facing device runtime and Pi-hosted admin API
* SQLite for local durable storage
* Cargo test tooling for Rust tests
* `just` for project command orchestration

Do not introduce Make or a `Makefile`. Use a `Justfile` for documented project workflows once commands exist.

Rust uses the stable toolchain declared in `rust-toolchain.toml` with `rustfmt` and `clippy` components. Cargo environment overrides live in `.cargo/config.toml`, including forced-empty host C/C++ flags for native dependency consistency.

The parent/admin dashboard source lives under `admin-ui/` and is built with Svelte + Vite. The static output served by `tcube-pi-admin` lives under `admin-ui/build/`. Use `pnpm` for every admin UI and JavaScript workflow; the Pi deployment consumes only the built static files and does not run Node. The admin UI styling system is Tailwind CSS v4 through Vite plus local CSS layers in `admin-ui/src/styles.css`; reusable dashboard surfaces live as Svelte components under `admin-ui/src/components/`.

All Rust changes must follow the [Rust Guide](rust-guide.md), including module boundaries, async and database rules, hardware isolation, and required quality gates.

## Runtime Split

`tcube-pi` is the Rust runtime.

It owns:

* GPIO event handling
* Button-to-mode mapping
* Local response selection
* Audio playback trigger
* LED effect trigger
* Fast event enqueueing or logging
* Hardware-facing safety behavior
* Approved content package caching and atomic activation
* Pi-hosted setup, authentication, content, media, and administration APIs
* Local browser asset and media serving
* Local operational state, content metadata, and event storage

The Mac mini is an external LLM and speech-generation host.

It owns local Voxtral, Vietnamese VITS, future transcription, categorization, summaries, and parent insights in separate repositories or services. `tcube-pi` may call those services over authenticated HTTPS for generated drafts, but the Mac mini is not the canonical store for cube configuration, accounts, content metadata, or usage events. The cube must continue its child-facing and local administration behavior when the Mac mini is offline.

Future background analysis is optional and must remain separate from the button response path. It may be omitted without reducing the cube's core play, setup, content-management, or offline behavior.

It may include transcription, categorization, summarization, parent insights, and content suggestions, subject to the microphone and data-retention privacy rules.

## Button Response Pipeline

The critical path must stay small:

1. GPIO event is received.
2. Button mode is resolved.
3. A local deterministic response is selected.
4. Audio playback and LED effect start immediately.
5. The event is logged asynchronously.
6. Optional audio capture and AI analysis happen outside the response path.

The Rust runtime must not wait for:

* AI inference
* Transcription
* Dashboard requests
* Network access
* Weekly summary generation
* Long database writes
* Software update checks

## Data Boundaries

Cube-local SQLite is the canonical source of truth for setup state, local accounts and sessions, button mappings, content metadata, device settings, and operational events. The local filesystem stores approved media, generated or uploaded drafts, runtime manifests, and any staged content revisions.

Expected cube-local data areas:

* Button and setup/debug events
* Button mappings and device settings
* Content metadata, media paths, and activation state
* Admin accounts, role assignments, invitations, recovery codes, and trusted sessions
* Wi-Fi verification and recovery state
* Package staging, activation, and failure metadata where packaged updates remain in use
* Optional microphone captures and derived summaries only after privacy rules are defined

The Mac mini is an external compute dependency, not a state authority. A speech or future AI request sends only the minimum required input over authenticated HTTPS and returns an artifact or derived result for explicit storage and approval on the cube. A failed, slow, or unavailable Mac request must leave existing local content and child-facing behavior unchanged.

Local content changes use staged writes and explicit activation. The runtime reads only approved local content; incomplete downloads, failed generation requests, corrupt media, or interrupted writes must not replace the active content set.

Network synchronization, backup, reporting, and software update checks are asynchronous. They must use bounded retries and durable local queues where delivery matters, and they must never block button playback.

Audio capture is optional and sensitive. Raw audio, upload behavior, retention periods, derived data, access controls, deletion, and physical microphone indication must be defined before capture ships.

## Admin Authentication And Authorization

The admin service uses local accounts identified by unique usernames; email is not required. Passwords use scrypt, and browser sessions use hashed random tokens in secure HTTP-only `SameSite=Strict` cookies with a rolling 90-day expiry.

Each Pi-hosted admin instance manages one cube. Owners can generate one-time seven-day manager invitation links, manage owner-sensitive setup, and revoke access. Managers can administer assigned cube content but cannot invite users or change owner-sensitive setup.

Invitation codes and session tokens are stored only as SHA-256 hashes. Invitation codes are single-use, and password reset with a recovery code revokes every active session for the account.

## Command Runner

Use `just` for repo workflows.

Core validation and formatting recipes:

* `just check`
* `just test`
* `just fmt`
* `just fmt-check`
* `just lint`

Current runtime and deployment-development recipes:

* `just run-device-sim`
* `just run-device-sim-audio`
* `just run-device-pi`
* `just run-pi-admin`
* `just validate-pi-admin-caddy`
* `just measure-pi-admin`

Recipes should call the underlying tools directly, such as `cargo` or `caddy`.

Do not add duplicate workflow entrypoints in shell scripts, Makefiles, or Make-compatible files.

The release artifact format, device authentication, HTTPS trust, and flashing/install workflow are currently being redesigned for GitHub Releases and are tracked in `docs/tasks.md`.

## Architecture Principles

* Keep child-facing behavior deterministic.
* Keep the button response path local and short.
* Prefer boring process boundaries over distributed complexity.
* Use SQLite before introducing a server-side database.
* Keep AI out of the immediate interaction loop.
* Treat local audio and derived summaries as sensitive data.
* Keep hardware control code small, observable, and service-like.
* Prefer explicit interfaces between Rust runtime, SQLite, content files, admin services, and future background services.
