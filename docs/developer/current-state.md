# Current Project State

Last updated: 2026-07-01 (+07)

This file is the live implementation snapshot for agents. Keep it concise; do not append chronological session history.

## Current Focus

- `tcube-pi` is the standalone Raspberry Pi repository for T-Cube.
- Maintained binaries are `tcube-pi`, `tcube-pi-admin`, and `tcube-pi-admin-measure`.
- The repository owns the child-facing runtime, keyboard simulator, local SQLite state, Pi-hosted admin API, checked-in admin UI build, Caddy deployment files, default content assets, and Pi hardware smoke payloads.
- The next highest-impact work is target Pi validation, real GPIO input, MAX98357A I2S output validation, LED output, microphone privacy rules, and installed service validation.

## Implemented

- Keyboard simulator, local content-pack loading, deterministic response selection, local audio playback through `rodio`, and SQLite event logging.
- Pi-hosted Rust admin service with setup, authentication, content, media, generated speech, recent activity, static UI, and static media/content routes.
- Versioned admin API aliases under `/api/pi/v1/*` while legacy unversioned paths remain available.
- Local accounts with scrypt password hashing, secure session cookies, recovery codes, manager invitations, and owner/manager role checks.
- Parent-created recordings, uploads, and generated speech drafts under `data/audio/draft/...`; activation moves media to `data/audio/active/...`.
- Recent activity feed combines runtime button events and admin activity events.
- Per-content play counts are computed from local button events.
- Admin UI source is Svelte + Vite + Tailwind under `admin-ui/`, with static build output checked in under `admin-ui/build/`.
- Admin UI is split into focused views, shared components, and controller helpers for button config, generated speech health, and recording/upload decisions.
- Admin UI upload flow stages audio as Choose, Review, and Save Draft steps, then sends parents to Drafts for preview and explicit activation.
- Release workflow builds Linux arm64 bundles with Rust binaries, prebuilt admin UI, content, Caddy/systemd files, installer, and SHA-256 checksums.

## Not Complete

- Real Raspberry Pi GPIO backend and physical debouncing.
- Final LED output backend.
- MAX98357A I2S audio validation on target hardware.
- Mini USB microphone capture, retention, upload, and physical indicator privacy rules.
- Installed Pi systemd validation and boot-time behavior.
- USB OTG recovery and Wi-Fi rollback behavior.
- Pi resource measurements with `just measure-pi-admin`.
- Durable SQLite schema versioning beyond current create-if-missing migrations.
- Full flashable SD-card image artifacts.

## Durable Decisions

- Child-facing playback stays local, deterministic, and independent of network, AI, dashboard requests, or reporting work.
- Runtime, admin service, and measurement harness stay separate binaries backed by reusable library modules.
- Caddy remains the HTTPS browser boundary; Rust admin listens loopback-only.
- Admin UI uses relative API paths so it works behind Caddy without hardcoded backend URLs.
- Admin UI is mobile-first and dark-mode-only.
- Admin UI source intentionally uses `pnpm`; Node/pnpm are development-time tools and are not required on the Pi.
- Mac-hosted speech and AI workers stay outside this repository; generated speech is always an inactive draft until parent activation.
- Hardware inventory and physical assembly maintenance live in `docs/hardware/hardware-assembly.md`.
- Fresh Raspberry Pi OS Lite setup and release-bundle installation live in `docs/hardware/pi-os-lite-install.md`.
- Release version bumps happen before tags; release workflows verify manifest/tag consistency instead of mutating manifests after tag push.

## Known Issues

- Physical GPIO, I2S, LED, and installed service behavior still need target-hardware validation.
- `src/server/handler.rs` still exists; inspect current route ownership before adding admin endpoints.
- Existing SQLite content package and failure tables remain after device-sync removal; schema cleanup needs a separate migration decision.
- Password change and session revocation controls are visually present in settings but disabled because local API contracts are not implemented.

## Validation

For Rust changes:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

For admin UI changes:

```sh
just build-admin-ui
just check-admin-ui
just test-admin-ui-unit
just test-admin-ui-mobile
```

Latest broad validation recorded on 2026-07-01 included `just check`, `just test`, `just check-admin-ui`, `just build-admin-ui`, `just test-admin-ui-unit`, and `just test-admin-ui-mobile`.

Latest documentation consolidation on 2026-07-01 removed redundant feature/auth/package/inventory docs, moved hardware and Pi install docs under `docs/hardware/`, and optimized default agent context routing.
