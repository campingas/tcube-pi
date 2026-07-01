# VISION.md - tcube-pi

This file is the product contract for `tcube-pi`. It explains why the repository exists and what every runtime, API, admin UI, and deployment change must preserve.

Related docs:

- Current implementation state: `docs/developer/current-state.md`
- Architecture and data boundaries: `docs/developer/architecture-guide.md`
- Admin UI design and copy: `docs/developer/branding-guide.md`

## One Sentence

`tcube-pi` is the brain of the T-Cube device: a Rust runtime on Raspberry Pi Zero 2 W that handles physical button presses, plays local audio, logs what happened, and exposes a local parent admin interface on the home network with no cloud dependency.

## Non-Negotiable Constraints

- Child-facing playback is local, deterministic, and immediate; normal button-to-audio latency must stay under 100 ms.
- The child has no screen; the child interface is physical buttons, audio, and hardware feedback.
- The parent admin UI is local, authenticated, mobile-first, and never part of the child's interaction path.
- Content generated, uploaded, or recorded by a parent remains a draft until explicitly activated.
- The Pi must keep working when the Mac, AI workers, internet, or future sync services are unavailable.

## Repository Responsibility

`tcube-pi` owns:

- `tcube-pi`: child-facing runtime and keyboard simulator.
- `tcube-pi-admin`: Pi-hosted local admin API and static admin UI server.
- `tcube-pi-admin-measure`: latency harness for proving admin load does not harm button response.
- Admin UI source and checked-in static build output.
- Local SQLite state, content/media storage conventions, Caddy/systemd deployment files, and Pi hardware smoke payloads.

This repository is not:

- A cloud service.
- A multi-device content platform.
- A child-facing screen UI.
- A Mac model worker or TTS service.
- A general hosted backend for the broader T-Cube ecosystem.

## Two-Surface Model

```text
Child presses button
       |
       v
  tcube-pi runtime
  local content pack
  deterministic selection
  audio playback < 100 ms
  async SQLite event log
  LED/hardware feedback

Parent opens browser
       |
       v
  Caddy HTTPS
       |
       v
  tcube-pi-admin
  /api/pi/v1/*
  static admin UI
  auth + session cookies
  content drafts and activation
```

The runtime owns the child's experience. The admin service can manage setup and content, but it must not block or slow the runtime's button path.

## Users

Parents and caregivers use the admin UI to bootstrap a local owner account, configure the five buttons, add/review/activate content, and monitor recent activity.

Developers use this repository to maintain the Rust runtime, admin API, local storage, release packaging, and Pi deployment path. `docs/developer/current-state.md` is the live implementation snapshot; `docs/developer/architecture-guide.md` is the technical architecture reference.

The Pi itself is the runtime target. Every runtime decision must respect Pi Zero 2 W constraints: limited resources, hardware I/O, local storage, and low-latency playback.

## Architecture Principles

1. The runtime owns interaction time; admin, AI, network, reporting, and sync work stay outside the button response path.
2. Local state on the Pi is authoritative; external services can contribute drafts or schedules but cannot bypass parent activation.
3. Activation is the safety gate; the child hears only active local content.
4. Authentication protects the admin service; the runtime has no auth surface.
5. Observability must not become surveillance; button/content events are logged, but voice, location, and biometric data require explicit privacy rules before implementation.
6. External sync remains deferred until publisher, hosting, auth, signing, rollback, and privacy boundaries are defined.

## Success State

A parent sets up T-Cube, hands it to a child, and a button press plays the chosen sound instantly. The parent opens the local dashboard, sees the event, adds a new recording, activates it, and the child hears exactly that new sound on the next press.

No third-party account is required. No internet is required for the child. No data leaves the home unless a parent explicitly enables a future feature with defined privacy boundaries.
