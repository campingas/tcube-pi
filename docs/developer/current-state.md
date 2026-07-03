# Current Project State

Last updated: 2026-07-02 (+07)

This file is the live implementation snapshot for agents. Keep it concise; do not append chronological session history.

## Current Focus

- `tcube-pi` is the standalone Raspberry Pi repository for T-Cube.
- Maintained binaries are `tcube-pi`, `tcube-pi-admin`, and `tcube-pi-admin-measure`.
- The repository owns the child-facing runtime, keyboard simulator, local SQLite state, Pi-hosted admin API, checked-in admin UI build, Caddy deployment files, default content assets, and Pi hardware smoke payloads.
- The next highest-impact work is target Pi validation, real GPIO input, MAX98357A I2S output validation, LED output, microphone privacy rules, and installed service validation.

## Implemented

- Keyboard simulator, local content-pack loading, deterministic response selection, local audio playback through `rodio`, and SQLite event logging.
- SoundBox button mode with a fixed catalog of six built-in melodies (three bedtime, three retro gaming, all public domain) synthesized at playback time by `src/hardware/soundbox.rs`; content-pack responses reference them as `audio_path: "builtin:<slug>"`, parents can only toggle catalog sounds per button (stored in `soundbox_selections`), and the admin API serves catalog list/toggle plus a synthesized WAV preview under `/api/pi/v1/content/soundbox/{slug}/preview`. Recording, uploads, and generation stay rejected for the `soundbox` type.
- Pi-hosted Rust admin service with setup, authentication, content, media, generated speech, recent activity, static UI, and static media/content routes.
- Versioned admin API aliases under `/api/pi/v1/*` while legacy unversioned paths remain available.
- Local accounts with scrypt password hashing, secure session cookies, recovery codes, manager invitations, and owner/manager role checks.
- Parent-created recordings, uploads, and generated speech drafts under `data/audio/draft/...`; activation moves media to `data/audio/active/...`.
- Recent activity feed combines runtime button events and admin activity events.
- Per-content play counts are computed from local button events.
- Admin UI source is Svelte + Vite + Tailwind under `admin-ui/`, with static build output checked in under `admin-ui/build/`.
- Admin UI is split into focused views, shared components, and controller helpers for button config, generated speech health, and recording/upload decisions.
- Admin UI upload flow stages audio as Choose, Review, and Save Draft steps, then sends parents to Drafts for preview and explicit activation.
- Focus routine settings are stored locally in SQLite and exposed under `/api/pi/v1/setup/pomodoro`; managers can view the state, owners can save/validate it, and the runtime skips the Pomodoro shortcut until the saved settings are enabled and validated.
- The runtime includes Pomodoro routine orchestration with generated `rodio` focus audio and transition chimes, silent breaks, and a tested Top + Front left + Front right hold recognizer for the future GPIO backend; the simulator exposes `p` as the manual routine shortcut.
- Release workflow builds Linux arm64 bundles with Rust binaries, prebuilt admin UI, content, Caddy/systemd files, installer, and SHA-256 checksums.
- The release installer exports Caddy's internal root CA to `/opt/tcube/ca/root.crt`; Caddy serves it at `/ca/root.crt` over both HTTPS and a port-80 HTTP listener that otherwise redirects to HTTPS, and the installer prints per-platform certificate trust steps (macOS, Linux, iPhone/iPad, Android).
- `tcube-mdns-alias.service` runs `/opt/tcube/bin/tcube-mdns-alias` (`avahi-publish` from `avahi-utils`) so `https://tcube.local/` resolves when the Pi hostname is not `tcube`; the installer also injects the LAN IP and `<hostname>.local` into the Caddy site list and reports mDNS status.
- The admin UI login screen includes a collapsible "Secure this device" card linking to `/ca/root.crt` with per-OS trust steps; it auto-expands when the page is opened from an IP-literal origin (`isIpLiteralHost` in `admin-ui/src/view-utils.ts`).
- Admin API integration tests live in `src/server/tests.rs` (moved from the former `src/server/handler.rs`); API routes register once in `src/server/routes/mod.rs` through a legacy-plus-versioned dual table.
- Admin UI dark theme uses the warm graphite palette documented in `docs/developer/branding-guide.md` (updated 2026-07-02); all status colors come from tokens in `admin-ui/src/styles.css`.

## Not Complete

- Real Raspberry Pi GPIO backend and physical debouncing.
- Physical Pomodoro three-button hold activation and cancellation still depend on the pending real GPIO input backend; the gesture state machine is implemented and unit-tested, but the desktop simulator uses the `p` shortcut.
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
- Caddy listens on LAN interfaces for browser traffic; local development phone testing should use `just run-pi-admin-lan-caddy` and `https://<host-lan-ip>:8443/`, with `TCUBE_LAN_ADDRESS=<host-lan-ip>` when automatic LAN IP detection is wrong. The release installer injects the current detected Pi LAN IP into `/etc/caddy/Caddyfile` so installed Pi access can use `https://<pi-lan-ip>/` when DHCP has not changed.
- Admin UI uses relative API paths so it works behind Caddy without hardcoded backend URLs.
- Admin UI is mobile-first and dark-mode-only.
- Admin UI source intentionally uses `pnpm`; Node/pnpm are development-time tools and are not required on the Pi.
- Mac-hosted speech and AI workers stay outside this repository; generated speech is always an inactive draft until parent activation.
- Hardware inventory and physical assembly maintenance live in `docs/hardware/hardware-assembly.md`.
- Fresh Raspberry Pi OS Lite setup and release-bundle installation live in `docs/hardware/pi-os-lite-install.md`.
- GitHub `dev` is the default development branch for pull requests; `main` is the stable branch used for release-ready promotion and raw install bootstrap URLs.
- Release version bumps happen before tags; release workflows verify manifest/tag consistency instead of mutating manifests after tag push.

## Known Issues

- Physical GPIO, I2S, LED, and installed service behavior still need target-hardware validation.
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

Latest broad validation recorded on 2026-07-02 included `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (67 passed), `just build-admin-ui`, `just check-admin-ui`, `just test-admin-ui-unit` (15 passed), and `just test-admin-ui-mobile` (12 passed).

Deploy script validation for the installer trust/mDNS work used `bash -n`, `shellcheck` on `deploy/pi-release/install-on-pi` and `deploy/pi-admin-caddy/tcube-mdns-alias`, `caddy validate` on the deployment Caddyfile (including the installer's address injection), and a local Caddy run confirming `/ca/root.crt` serves with `application/x-x509-ca-cert` while other HTTP requests redirect to HTTPS.

Latest documentation consolidation on 2026-07-01 removed redundant feature/auth/package/inventory docs, moved hardware and Pi install docs under `docs/hardware/`, and optimized default agent context routing.
