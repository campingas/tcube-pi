# Architecture Guide

This guide captures durable technical architecture. For live status, use `current-state.md`; for product constraints, use `../../VISION.md`.

## Core Constraint

Child-facing button feedback must be immediate, deterministic, offline-first, and never blocked by AI, dashboard, network, sync, or reporting work.

## Stack

| Layer | Technology | Notes |
| --- | --- | --- |
| Runtime | Rust stable | Toolchain pinned by `rust-toolchain.toml` |
| Local state | SQLite via `rusqlite` | Canonical cube-local state |
| Audio playback | `rodio` | Local playback backend |
| Admin API | Axum/Tokio | Pi-hosted local HTTP behind Caddy |
| Admin UI | Svelte + Vite + Tailwind CSS | Built files served by `tcube-pi-admin` |
| Admin UI package manager | `pnpm` | Development-time only, not installed on the Pi |
| HTTPS boundary | Caddy | Browser-facing TLS for the local admin UI |
| Task runner | `just` | Documented workflows |

## Binaries

- `tcube-pi`: child-facing runtime and simulator.
- `tcube-pi-admin`: local parent admin API and static UI server.
- `tcube-pi-admin-measure`: latency harness for button handling under admin API load.

Keep binary entry points thin. Reusable behavior belongs under `src/`.

## Runtime/Admin Split

```text
Button press
  -> runtime resolves local button mapping
  -> runtime selects active local content
  -> runtime starts audio/hardware feedback
  -> runtime logs event asynchronously

Parent browser
  -> Caddy HTTPS
  -> tcube-pi-admin loopback service
  -> local API, static UI, setup, auth, drafts, activation
```

The runtime and admin service may share SQLite and filesystem state, but they must not share execution context or block each other. Admin writes must be short, staged, and safe for concurrent runtime reads.

## Button Response Path

Only these actions belong before audio starts:

1. Receive GPIO or simulator event.
2. Resolve button mode from local mapping.
3. Select active local response deterministically.
4. Start audio and hardware feedback.
5. Queue or perform non-blocking event logging.

Do not add network calls, AI inference, dashboard requests, update checks, sync work, long database writes, or reporting work to this path. Any proposed change that adds latency here requires explicit review and measurement.

## Admin API Pattern

- Use Axum routes with typed extractors for state, JSON, path/query params, multipart fields, and cookies.
- Keep blocking SQLite and filesystem work out of async executor threads with the route-layer blocking pattern already used in the codebase.
- Return JSON errors with stable status codes for API failures.
- Serve static admin UI, media, and bundled content through Tower/static-file helpers with path traversal rejection.
- Keep multipart audio upload limits explicit; audio larger than the product limit must be rejected before storing a draft.
- Avoid N+1 content queries; load shared lookup state once and add indexes for new high-volume filters.

## Data Boundaries

SQLite on the Pi is authoritative for:

- Button events and setup/debug events.
- Button mappings and device settings.
- Content metadata, media paths, activation state, and play counts.
- Admin accounts, sessions, roles, invitations, and recovery codes.
- Wi-Fi/setup state.
- Future package staging only after sync requirements are defined.

Filesystem media boundaries:

- Active child-playable media lives under `data/audio/active/...`.
- Parent-created drafts live under `data/audio/draft/...`.
- The runtime reads only active content.

Failed downloads, corrupt media, interrupted writes, and generated/uploaded drafts must never replace active content until explicit parent activation succeeds.

## Auth Boundary

The admin service uses local accounts, scrypt password hashing, secure HTTP-only cookies, hashed session/invitation/recovery tokens, and owner/manager role checks.

The runtime has no authentication surface. Do not add auth logic to the child-facing runtime.

## External Compute

Mac-hosted TTS and AI workers are optional compute providers, not state authorities. They may create draft audio or proposed schedules through the admin API, but the Pi remains authoritative and parent activation remains mandatory.

The Pi must behave correctly when external providers are offline, slow, or misconfigured.

## Hardware Architecture

Current v1 target hardware is documented in `../hardware/hardware-assembly.md`.

Hardware control code should stay isolated behind small runtime abstractions so desktop simulator tests do not require Pi hardware. GPIO, I2S, LED, IMU, and microphone work must remain measurable and simulator-safe.

Audio capture is sensitive. Before microphone capture ships, define retention, deletion, upload boundaries, physical indicator behavior, and parent consent.

## Deployment

- Rust admin listens on loopback HTTP.
- Caddy exposes HTTPS to browsers and phones.
- Release bundles install under `/opt/tcube`, `/etc/tcube`, `/var/lib/tcube`, and systemd/Caddy locations.
- Fresh Pi setup lives in `../hardware/pi-os-lite-install.md`.
- Full flashable SD-card images remain deferred until hardware, Caddy, systemd, GPIO, I2S, and resource validation are complete.
