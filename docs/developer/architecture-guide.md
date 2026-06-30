# Architecture Guide — tcube-pi

> This guide defines the intended technical architecture for the `tcube-pi` repository.
>
> The product constraint from `VISION.md` is the primary architectural driver:
> **child-facing button feedback must be immediate, deterministic, offline-first, and never blocked by AI, dashboard, network, or reporting work.**
>
> For visual design, colour tokens, and admin UI component direction → `docs/developer/branding-guide.md`
> For what the system currently does → `docs/developer/FEATURES.md`
> For why it exists → `VISION.md`

---

## Preferred Stack

### Hardware — Pi device

| Component               | Part                              | Role                                                         | Version |
|-------------------------|-----------------------------------|--------------------------------------------------------------|---------|
| Main computer           | Raspberry Pi Zero 2W              | Runs the Rust runtime and admin service                      | v1      |
| Buttons                 | MakerEdu MKE-M02 RGB ×5          | Physical input and RGB LED feedback via GPIO                 | v1      |
| Amplifier               | MAX98357A I2S Class-D 3W          | Digital audio output via I2S bus                             | v1      |
| Speaker                 | Mini 3W 8Ω with enclosure         | Plays speech, sounds, and device feedback through base grille| v1      |
| IMU                     | MPU-6050 GY-521                   | Detects orientation, rotation, movement, and impact via I2C  | v1      |
| Microphone              | INMP441 I2S MEMS                  | Voice capture for repeat-after-me and teacher mode           | v2      |

The INMP441 (v2) shares the I2S bus with the MAX98357A in full-duplex mode. GPIO18 (BCLK) and GPIO19 (LRCLK) are shared clock lines driven by the Pi as master. The MAX98357A receives audio data on GPIO21; the INMP441 sends captured audio on GPIO20. A custom device tree overlay is required — the standard `googlevoicehat-soundcard` and `max98357a` overlays conflict and cannot be loaded simultaneously. See `docs/hardware/i2s-fullduplex.md` for the `.dts` source and wiring table.

Physical placement rule: the speaker fires downward through the base grille; the INMP441 sits on the top face. This separation reduces acoustic echo at child interaction distances without requiring software AEC in v2. AEC is deferred to a later firmware revision pending real-world testing.

### Hardware — Mac on the same network

The Mac is an external compute dependency, not a state authority. It runs two optional services that the parent installs:

- A **TTS model** (Voxtral for English, French, Spanish; a Vietnamese VITS model for Vietnamese; a Mandarin model for Chinese) that generates speech from text and returns audio files to the Pi for staging and review.
- A **general-purpose LLM** (e.g. Qwen 3) that reads `learning.md`, audits the Pi's local content database, and produces a curated content schedule pushed back to the Pi as an updated playlist. This is the learning maintenance loop.

Reference machine: Mac Studio M4 Ultra (256 GB unified memory). Any Apple Silicon Mac on the same network with sufficient memory to run the chosen models is sufficient. The Pi runtime must continue its child-facing and admin behavior when the Mac is offline. No Mac service is ever in the button response path.

### Software

| Layer          | Technology                                   | Notes                                                      |
|----------------|----------------------------------------------|------------------------------------------------------------|
| Runtime        | Rust (stable, `rust-toolchain.toml`)         | Child-facing device runtime and Pi-hosted admin API        |
| Local storage  | SQLite via `rusqlite`                         | Canonical source of truth for all cube-local state         |
| Audio playback | `rodio`                                      | Local audio playback backend                               |
| Admin UI       | Svelte + Vite + Tailwind CSS v4               | Served as same-origin static files by `tcube-pi-admin`     |
| Package manager| `pnpm`                                       | All admin UI and JavaScript workflows                      |
| Task runner    | `just`                                       | All repo workflows — no Makefile, no shell script wrappers |
| HTTPS proxy    | Caddy                                        | Terminates TLS for the admin service on the home network   |
| Linting        | `clippy`, `rustfmt`                          | Enforced in CI and locally via `just lint` / `just fmt`    |

Rust uses the stable toolchain declared in `rust-toolchain.toml` with `rustfmt` and `clippy` components. Cargo environment overrides live in `.cargo/config.toml`, including forced-empty host C/C++ flags for native dependency consistency.

The admin UI source lives under `admin-ui/`. The static output served by `tcube-pi-admin` lives under `admin-ui/build/`. The Pi deployment consumes only the built static files — Node does not run on the Pi. Styling uses Tailwind CSS v4 through Vite plus local CSS layers in `admin-ui/src/styles.css`. Reusable dashboard surfaces live as Svelte components under `admin-ui/src/components/`. The visual design system — Wada palette tokens, dark-mode surface mapping, type scale, component direction, and copy voice — is defined in `docs/developer/branding-guide.md`. New components must reference that guide before introducing any colour, spacing, or typography value.

Admin UI views are designed mobile-first. The phone layout is the primary implementation target for setup, button configuration, content review, and quick actions. Desktop and tablet layouts may add columns or wider constraints, but they must not introduce desktop-only navigation or require a larger viewport to complete any parent workflow.

All Rust changes must follow the [Rust Guide](rust-guide.md), including module boundaries, async and database rules, hardware isolation, and required quality gates.

---

## Binary Responsibilities

`tcube-pi` is a single Rust workspace with three maintained binaries.

### `tcube-pi` — child-facing runtime

The runtime is the product. It owns:

- GPIO event handling and button-to-mode mapping
- Deterministic local response selection from the approved content set
- Audio playback trigger and LED effect trigger
- Fast asynchronous event logging to SQLite
- Hardware-facing safety behavior
- Approved local content caching and atomic activation

The runtime must not wait for AI inference, transcription, dashboard requests, network access, or any long-running operation. See the Button Response Pipeline section for the exact constraint.

### `tcube-pi-admin` — parent's local admin service

The admin service owns:

- Loopback HTTP service behind Caddy HTTPS
- Tower-backed static serving for the admin UI build output under `admin-ui/build/`
- Tower-backed static media serving from the configured media and content roots
- All `/api/pi/v1/` endpoints: status, authentication, setup, content management, media upload, and generated speech drafts
- Local account management: owner bootstrap, password hashing (scrypt), session tokens, invitation codes, recovery codes, and role checks

The admin service shares the SQLite database and filesystem with the runtime. It must never block the button response path. Any write that could contend with runtime reads must use SQLite WAL mode, avoid write amplification on read-heavy paths, and be structured to resolve quickly from the runtime's perspective.

### `tcube-pi-admin-measure` — latency harness

Records button-handling latency under concurrent admin API load. Reports p50, p95, p99, max latency, and admin request success/failure counts as JSON. Run before any architecture change that touches the I2S audio path, the SQLite schema, or the content activation flow. Provides the evidence that the admin service does not degrade the child-facing runtime.

---

## Admin API Implementation Pattern

The admin API uses Axum routes with typed extractors for state, JSON bodies, path params, query params, multipart uploads, and `tcube_session` cookies. Blocking SQLite work and filesystem mutations stay outside async executor threads through the route-layer blocking helper.

Static admin UI, media, and bundled content responses use Tower static file services. The admin UI route keeps SPA fallback to `index.html`; media and content routes keep path traversal rejection and JSON error bodies.

Multipart audio uploads read the audio field incrementally and reject audio larger than 25 MB before storing the draft. Keep the route body limit high enough for a valid 25 MB audio field plus multipart overhead, but keep the audio-field limit as the product boundary.

Content inventory and list endpoints must avoid N+1 database reads. Load shared lookup state once per request, keep content query paths indexed, and add targeted indexes with any new high-volume content filter or ordering pattern.

---

## Runtime Split

The Pi runs two concurrent processes: the child-facing runtime and the admin service. They share state through SQLite and the local filesystem. They must never share execution context or block each other.

```
Child presses button
       │
       ▼
  [tcube-pi runtime]        ← this process owns the child's experience
  GPIO event received
  Button mode resolved
  Local response selected   ← deterministic, from approved content only
  Audio + LED start         ← target: < 100 ms button-to-audio
  Event logged async        ← never blocks playback
       │
       (no network, no AI, no admin service involvement)

Parent opens browser
       │
       ▼
  [Caddy HTTPS :443]
       │
       ▼
  [tcube-pi-admin]          ← this process owns the parent's control surface
  /api/pi/v1/* endpoints
  admin UI static files
  content drafts and activation
  SQLite reads and writes   ← WAL mode, never contends with runtime
```

The Mac on the home network connects only to the admin service — never to the runtime. Generated speech and LLM-curated content schedules arrive as drafts, require explicit parent activation, and are pushed to the Pi's local content store before the runtime ever sees them.

---

## Button Response Pipeline

The critical path must stay small. These six steps are the only things that happen before audio plays:

1. GPIO event is received.
2. Button mode is resolved from the local mapping.
3. A deterministic local response is selected from the approved content set.
4. Audio playback and LED effect start immediately.
5. The event is logged asynchronously to SQLite.
6. Optional audio capture and AI analysis happen entirely outside this path.

The runtime must never wait for:

- AI inference or transcription (on the Mac or anywhere else)
- Dashboard HTTP requests
- Network access of any kind
- SQLite writes that are not the async event log
- Software update checks
- Weekly summary generation or any reporting work

If a proposed feature would add a step between GPIO event receipt and audio playback start, it requires explicit architectural review and a latency measurement before merging.

---

## The Learning Layer

The learning layer is the optional intelligence that makes T-Cube's content get smarter over time. It runs on the Mac — never on the Pi — and communicates with the Pi only through the admin service.

### `learning.md`

The LLM's operating document. Describes the child: age, grade, languages, pace, learning goals. The parent writes and edits it. It is the instruction set that turns a generic LLM into this child's learning maintainer. It lives in the repo under `docs/notes/learning.md` and is read by the Mac LLM at the start of each curation loop.

### The curation loop

The loop is demand-triggered, not scheduled. It runs when:

- The parent adds new content (upload, recording, generated speech)
- The parent or child explicitly requests a content review
- The parent judges that the child's level has advanced enough to warrant new material

The loop sequence:

1. The Mac LLM reads `learning.md` and queries the Pi admin API for the current content inventory (what sounds are active, their play counts, their activation dates).
2. The LLM applies the learning principles in `learning.md` — spacing intervals, difficulty gradient, language balance — and produces a curated schedule: which content to keep active, which to deactivate, which new content to generate.
3. New speech is generated via the Mac TTS model and pushed to the Pi admin API as inactive drafts.
4. The parent reviews the proposed changes in the admin UI and activates or rejects each item.
5. The activated schedule is pushed to the Pi's local content store. The runtime reads it on the next button press.

The loop produces two outputs: an updated content playlist on the Pi, and a statistics report (words heard, repetition counts, estimated retention) visible to the parent in the admin UI. Both outputs are local. No data leaves the home network.

### Mic capture and voice interaction (v2)

The INMP441 mic enables two new interaction modes in v2:

- **Repeat-after-me mode** — the cube plays a natural sound then speaks a word; it waits for the child to name it aloud; the Mac LLM evaluates the response and adjusts the content schedule.
- **Teacher mode** — the LLM reviews lessons with the child, records attempts to explain concepts back, plays them back, and guides improvement.

Audio capture is sensitive data. The following rules apply before any capture ships:

- Raw audio is never stored on the Pi beyond the duration of a single session without explicit parent consent.
- Audio sent to the Mac LLM travels over authenticated HTTPS on the home network only — it never reaches an external service.
- The physical mic must have a hardware or firmware mute that the parent controls.
- Retention periods, access controls, and deletion workflows must be defined and implemented before capture is enabled by default.
- The latency harness must be re-run after any I2S full-duplex firmware change to confirm the button response path is unaffected.

---

## Data Boundaries

### Cube-local SQLite (canonical state)

The SQLite database on the Pi is the canonical source of truth for all cube-local state. No external service overrides it. The Mac is an input source — it contributes drafts and schedules — but it is not authoritative.

Cube-local data areas:

- Button events and setup/debug events
- Button mappings and device settings
- Content metadata, media paths, and activation state
- Admin accounts, role assignments, invitations, recovery codes, and trusted sessions
- Wi-Fi verification and recovery state
- Future package staging and failure metadata only after external sync requirements are clarified
- Learning schedule metadata and play-count statistics
- Optional microphone captures and derived summaries — only after privacy rules are defined and implemented

### Local filesystem

The Pi filesystem stores:

- Approved active media under `data/audio/active/{content_type}/` (the runtime reads only from this set)
- Recorded, uploaded, and generated drafts under `data/audio/draft/{content_type}/` (staged, never read by the runtime until activated)
- Runtime content manifests

### The Mac (external compute, not state authority)

A TTS or LLM request sends only the minimum required input over authenticated HTTPS and returns an artifact (audio file or content schedule) for explicit storage and parent approval on the Pi. A failed, slow, or unavailable Mac request leaves existing local content and child-facing behavior unchanged. The Pi never blocks on a Mac response.

### Content safety guarantee

The runtime reads only the approved local content set. Incomplete downloads, failed generation requests, corrupt media, and interrupted writes must not replace the active content set. All content changes use staged writes and require explicit activation through the admin UI before the runtime sees them.

---

## Admin Authentication and Authorization

The admin service uses local accounts identified by unique usernames. Email is not required.

- **Passwords**: scrypt hashing. No plaintext storage, no reversible encryption.
- **Sessions**: hashed random tokens in secure HTTP-only `SameSite=Strict` cookies with rolling 90-day expiry. Session timestamp writes are throttled to avoid turning repeated dashboard reads into repeated SQLite writes.
- **Invitation codes and session tokens**: stored only as SHA-256 hashes. Invitation codes are single-use.
- **Password reset**: using a recovery code revokes every active session for the account.
- **Roles**: Owner — full access including setup, invitation management, and role-sensitive endpoints. Manager — content administration only, no owner-sensitive setup, no invitation creation.
- **Scope**: each Pi-hosted admin instance manages one cube. There is no multi-device management in this repository.

---

## Command Runner

Use `just` for all repo workflows. Do not add duplicate entrypoints in shell scripts or Makefiles.

### Validation and formatting

```
just check
just test
just fmt
just fmt-check
just lint
```

### Runtime and development

```
just run-device-sim          # keyboard simulator, no audio
just run-device-sim-audio    # keyboard simulator with local audio backend
just run-device-pi           # full Pi runtime (requires hardware)
just run-pi-admin            # Pi-hosted admin service
just validate-pi-admin-caddy # validate Caddy config before deploying
just measure-pi-admin        # run the latency harness and report results
```

Recipes call the underlying tools directly (`cargo`, `caddy`). No wrapper abstractions.

> **Open:** Full flashable SD-card images and any future external content sync/update mechanism remain deferred until the hardware deployment path and sync requirements are clarified. Tracked in `docs/tasks.md`.

---

## Architecture Principles

These principles are ordered by priority. When two principles conflict, the higher one wins.

1. **Child-facing behavior is deterministic and immediate.** The button response path is local, short, and never blocked by any external dependency. This is non-negotiable.

2. **Local first, external optional.** The Pi works without the Mac, without the internet, and without any cloud service. Every external capability (TTS generation, LLM curation, future content transfer) adds value when present and removes nothing when absent. External content sync is deferred until the parent/device use case, update source, package format, auth model, rollback behavior, and privacy rules are defined.

3. **Activation gates the child's experience.** The runtime reads only content the parent has explicitly activated. Drafts, staged files, failed downloads, and corrupt media never reach the child. This is a child-safety guarantee, not a convenience feature.

4. **The Mac is a compute dependency, not a state authority.** The Pi's SQLite database is the canonical state. The Mac contributes artifacts and schedules. It never overwrites or bypasses the Pi's local state.

5. **Authentication is the admin's perimeter. The runtime has no perimeter.** The admin service enforces scrypt, secure cookies, role checks, and HTTPS. The runtime runs in a closed hardware context where the perimeter is physical. Do not add authentication logic to the runtime.

6. **Audio capture is sensitive. Define rules before shipping.** Raw audio, retention, access controls, deletion, and physical mic indication must be specified and implemented before any capture feature is enabled.

7. **Keep hardware control code small, observable, and isolated.** GPIO, I2S, and LED code lives behind explicit hardware abstraction interfaces. It is tested through the simulator on desktop without requiring physical hardware.

8. **Prefer boring process boundaries over distributed complexity.** SQLite before a database server. Local files before object storage. Direct HTTP before a message queue. Add complexity only when the simpler approach has been proven insufficient.

9. **Observability without surveillance.** The SQLite event log records button presses and content playback. It does not record voice, location, or biometric data without explicit parent consent and defined retention rules.
