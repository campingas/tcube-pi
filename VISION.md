# VISION.md — tcube-pi

> This file explains **why** this repository exists, **what** it is responsible for, and **how** its two main parts relate to each other and to the rest of the T-Cube ecosystem.
> It is not a spec — it is the north star that keeps every Rust module, every API endpoint, and every admin UI component honest.
>
> Branding tokens, type scale, colour, and component direction → `docs/developer/branding-guide.md`
> Full feature inventory → `docs/developer/FEATURES.md`
> Learning science operating standard → `docs/notes/learning.md`

---

## The one sentence

> `tcube-pi` is the brain of the T-Cube device: a Rust runtime that runs on the Pi Zero 2W, handles every button press, plays every sound, and exposes a local admin interface so a parent can manage content, configure the cube, and monitor what their child has heard — entirely on the home network, with no cloud dependency.

Every module, binary, and admin UI component must be able to defend its existence against this sentence. If it doesn't serve the runtime or the parent's local control, it does not belong here.

---

## Why this repository exists

T-Cube the device is defined by two constraints that this repository must never violate:

**No cloud dependency for the child.** When a child presses a button, the sound plays from local storage. The Pi does not call home. It does not authenticate. It does not wait for a network response. The interaction latency between button press and audio output must be imperceptible — under 100ms in all normal operating conditions. This is a hard product requirement, not a performance target.

**No screen for the child.** The child's interface is five physical buttons and an audio output. The admin UI is a parent tool. It is accessed from a phone or laptop on the home network. It is never the child's interface. Any feature that would require the child to look at a screen is out of scope for this repository.

These two constraints define the architecture: a fast local Rust runtime for the child's experience, and a separate local web service for the parent's control surface, both running on the same Pi, both entirely self-contained.

---

## What this repository contains

`tcube-pi` is a single Rust workspace with three maintained binaries and one admin UI build:

### `tcube-pi` — the child-facing runtime

The runtime is the product. It loads a local content pack, maps five buttons to their configured behaviors, selects and plays audio responses deterministically, logs button events to SQLite, and manages LED feedback state. It runs headless on the Pi Zero 2W from boot, with no UI, no network calls for playback, and no external dependencies at interaction time.

The runtime must be:
- **Fast** — button-to-audio latency under 100ms
- **Reliable** — no crashes on bad input, no silent failures on missing content
- **Auditable** — every button event logged to SQLite with enough context to reconstruct what a child heard and when
- **Testable on desktop** — the keyboard simulator and TUI allow full development without physical hardware

### `tcube-pi-admin` — the parent's local dashboard

The admin service is a parent tool. It runs as a loopback HTTP service behind Caddy HTTPS on the Pi, serves the admin UI as same-origin static files, and exposes a versioned local API (`/api/pi/v1/`) for all setup and content management operations.

The admin service must be:
- **Secure** — scrypt-hashed passwords, secure HTTP-only cookies, role-checked endpoints, HTTPS-only
- **Self-contained** — no external auth service, no cloud account, no third-party identity provider
- **Trustworthy** — a parent who installs this service must be able to understand and audit what it does with their data
- **Functional over decorative** — the admin UI is a tool, not a marketing surface

### `tcube-pi-admin-measure` — the latency harness

A measurement binary that records button-handling latency under concurrent admin API load. Its job is to prove that the admin service does not degrade the child-facing runtime. It reports p50, p95, p99, and max latency, plus admin request success/failure counts. It runs in CI and before any architecture change that touches the I2S audio path or the SQLite schema.

### Admin UI — the Svelte dashboard

A Svelte and Vite dashboard served as same-origin static files by `tcube-pi-admin`. It is the primary visual surface of this repository. It handles account bootstrap, login, content management, button configuration, setup prerequisites, and media upload. It communicates exclusively with the Pi admin API over relative paths, so it works behind the Caddy HTTPS boundary without a hardcoded URL.

The admin UI is **mobile-first**. Parents are expected to use it primarily from a phone on the home network, especially during setup and quick content changes. Desktop and tablet layouts are responsive enhancements, not separate product surfaces, and no admin flow may require a desktop viewport.

The admin UI is a **dark-mode-only** tool. It does not follow OS preferences. It does not have a light mode. The design rationale is in `docs/developer/branding-guide.md`.

---

## Who uses this repository

### Primary — Parents and caregivers (admin UI)

The parent's primary interaction with this codebase is through the admin UI. They arrive after building or buying a T-Cube, they need to:

1. Bootstrap a local owner account
2. Configure the five buttons (language, animals, music, setup help, disabled)
3. Upload, record, or generate content for each button
4. Review draft content before it goes live to the child
5. Monitor what the cube has played

The parent is not a developer. The admin UI must be usable by a technically-comfortable but non-expert adult. It should not require reading a manual. Every state — loading, error, success, empty — must be handled visibly and clearly.

### Secondary — Developers and contributors (runtime + API)

Developers who build on, contribute to, or fork this repository need to understand:
- The Rust workspace structure and binary responsibilities
- The SQLite schema and event logging patterns
- The admin API versioning and endpoint contracts
- The hardware abstraction layer and GPIO stub

For developers, the FEATURES.md is the reference. This VISION.md is the filter: if a proposed feature doesn't serve the child's interaction or the parent's local control, it needs a strong justification before it gets added.

### Tertiary — The Pi itself (runtime)

The runtime's primary "user" at interaction time is the hardware: the GPIO pins, the I2S bus, the audio output, the LED matrix. Every design decision in the runtime must account for the constraints of the Pi Zero 2W — limited RAM, single-core ARM, shared I2S bus between amp and mic. Correctness and latency are the only metrics that matter here.

---

## What this repository is not

**Not a cloud service.** There is no hosted backend, no SaaS, no managed database. The Pi is the server. The parent's home network is the network. If a feature requires an external API call during the child's session, it does not belong here.

**Not a content authoring platform.** The admin UI lets a parent manage content on a single device. It is not a multi-device content management system, a shared library, or a publishing platform. External content sync is deferred until the parent/device use case, trust model, update source, package format, rollback behavior, and privacy rules are defined.

**Not a child-facing UI.** The admin UI is locked behind authentication. It is never shown to the child. Any screen, menu, or interface the child might see is a hardware concern (LED patterns, audio cues) — not an admin UI concern.

**Not finished.** GPIO and I2S hardware integration, physical button debouncing, mic capture, and repeat-after-me mode are planned or in progress. This VISION.md evolves with the codebase — update it before changing the runtime's core interaction model or the admin UI's primary flows.

---

## The two-surface model

This repository produces two user-facing surfaces that must coexist without interfering:

```
Child presses button
       │
       ▼
  [tcube-pi runtime]
  local content pack
  deterministic selection
  audio playback < 100ms
  SQLite event log
  LED feedback
       │
       (no network, no auth, no delay)

Parent opens browser
       │
       ▼
  [Caddy HTTPS]
       │
       ▼
  [tcube-pi-admin]
  local API /api/pi/v1/
  static admin UI
  auth + session cookies
  content management
  setup flows
       │
       (loopback only, home network, never reaches child session)
```

These two surfaces share one SQLite database and one filesystem (the content/media root). They must never block each other. The latency harness (`tcube-pi-admin-measure`) exists specifically to verify this separation under load.

---

## Architecture principles

**1. The runtime owns the child's experience. The admin service owns nothing at interaction time.**
The admin service can read and write content at any time — but it must not degrade button-to-audio latency. Any write to the content pack or SQLite that could block the runtime must be queued or deferred.

**2. Local first, sync optional.**
The device works without a home Mac, without an LLM, and without any external sync service. Every optional layer (TTS generation, LLM curation loop, future content transfer) adds capability but removes nothing when absent. Future external content sync must clarify who publishes packages, where they are hosted, how devices authenticate, how rollback works, and what data can leave the home network before implementation.

**3. Activation gates everything.**
Content uploaded or generated by a parent exists as an inactive draft until explicitly activated. The child never hears unreviewed content. This is not a convenience feature — it is a child-safety guarantee.

**4. Authentication is the admin's perimeter. The runtime has no perimeter.**
The admin service enforces scrypt passwords, secure cookies, role checks, and HTTPS. The runtime enforces nothing — it runs in a closed hardware context where the perimeter is physical. Do not add authentication logic to the runtime.

**5. Observability without surveillance.**
The SQLite event log records what buttons were pressed and what content played. It does not record voice, location, or any biometric data. When the mic is added in v2, voice capture is processed locally on the Mac LLM — it is never stored on the Pi or sent to an external service without explicit parent consent.

---

## Relationship to the broader T-Cube ecosystem

```
tcube-landing/          ← marketing, family and maker audience
tcube-pi/               ← this repository
  ├── runtime           ← child's experience, Pi Zero 2W
  └── admin service     ← parent's local dashboard
tcube-tts/              ← Mac-hosted TTS authoring (Voxtral, VITS)
  └── learning.md       ← LLM operating document, managed by Mac LLM
```

`tcube-pi` is the edge. It is the only part of the ecosystem that runs on the device the child holds. Everything else — the landing page, the Mac LLM, the TTS pipeline, a future cloud admin — connects to it or informs it, but does not replace it.

---

## What success looks like

A parent finishes setting up their T-Cube. They hand it to their three-year-old. The child presses a button. A word plays instantly. The parent opens the admin dashboard on their phone and sees the event logged. They upload a new recording. They activate it. The child presses the button again. The new sound plays.

No internet connection was used. No account was created with a third party. No data left the home. The child heard exactly what the parent chose.

That is the success state this repository exists to produce.
