# Current Project State

Last updated: 2026-06-23 (+07)

## Current Focus

- `tcube-pi` is extracted as a standalone Rust repository for the Raspberry Pi side of T-Cube.
- The root package is `tcube-pi`, the Rust crate import path is `tcube_pi`, and the maintained binaries are `tcube-pi`, `tcube-pi-admin`, `tcube-pi-admin-measure`, and `tcube-pi-device-api`.
- The repository owns the child-facing runtime, keyboard simulator, local SQLite state, Pi-hosted admin API, checked-in static admin UI, content sync client/API compatibility code, Caddy deployment files, default content assets, and Pi hardware smoke payloads.
- The Mac-hosted TTS workers are intentionally not part of this repository. The checked-in `admin-ui/build/` is the static build output from the Svelte + Vite source in `admin-ui/` and is served by `tcube-pi-admin`; generated speech may call external HTTPS speech services for drafts.
- The admin UI now uses Tailwind CSS v4 through Vite with local neo-dashboard CSS layers and Svelte components for the top banner, status chips, button face matrix, selected button workspace, content lists, draft queue, add-content tabs, and terminal feedback strip.
- The Pi admin HTTPS boundary remains Caddy in front of loopback-only `tcube-pi-admin`: Rust listens on `127.0.0.1:8080`, and Caddy serves `https://tcube.local/` and `https://10.55.0.1/` with `tls internal`.

## Implemented

- Keyboard simulator for button development without Raspberry Pi hardware.
- Local audio playback through approved content assets.
- SQLite event logging and setup/debug event separation.
- Pi-hosted Rust admin API with setup, authentication, content, media, generated speech, and local static/media serving endpoints.
- Svelte + Vite + Tailwind admin UI source under `admin-ui/`, with checked-in static build output under `admin-ui/build/`.
- Local account, session, recovery-code, and manager-invitation primitives.
- Content sync API/client modules for staged package download, checksums, activation, acknowledgements, and failure reporting.
- Caddy deployment sketch and one-button Pi Zero bench smoke payload.
- Rust formatting, linting, and test workflows through `just`.
- GitHub Actions CI under `.github/workflows/ci.yml` runs Rust formatting, check, Clippy, tests, and admin UI pnpm install/check/build gates.
- Repository Rust workflow config uses `rust-toolchain.toml` for stable Rust with `rustfmt` and `clippy`, plus `.cargo/config.toml` to force inherited host C/C++ flags empty for native dependency consistency.

## Not Yet Complete

- Real Raspberry Pi GPIO backend validation.
- Final LED output backend.
- MAX98357A I2S audio validation from the Rust runtime on target hardware.
- Mini USB microphone capture, retention, upload, and physical indicator privacy rules.
- Installed Pi systemd validation for the final runtime.
- USB OTG recovery and Wi-Fi rollback behavior.
- Pi resource measurements with `just measure-pi-admin`.
- GitHub Releases workflow for flashable Pi artifacts.
- Release packaging in GitHub Actions.

## Known Issues

- `docs/developer/rust-guide.md` describes the desired future Axum and SQLite boundaries, but the current admin implementation still uses a fallback router with a hand-rolled request dispatcher and colocated SQL in `src/server/handler.rs`.

## Architectural Decisions

- Keep child-facing behavior local, deterministic, and independent of network or AI availability.
- Keep `tcube-pi` Rust binaries as thin entry points backed by reusable library modules under `src/`.
- Keep Caddy as the HTTPS browser boundary rather than terminating TLS inside the Rust admin service.
- Keep the admin UI as same-origin static assets. API calls use relative paths so the dashboard works behind Caddy at `https://tcube.local/` without a hardcoded backend URL.
- Use `pnpm` for every admin UI and JavaScript workflow; no Node runtime is required on the Pi.
- Keep admin UI API contracts in `admin-ui/src/api.ts` and inspect Rust handler tests before adding or changing endpoint request bodies. Do not guess backend fields from UI needs.
- Report setup dashboard URLs as standard HTTPS URLs without an explicit port, matching the Caddy deployment on port 443.
- Keep Mac-hosted speech and AI workers outside this repository; generated speech is a draft artifact until reviewed and activated locally.
- Keep `just` as the only documented command runner.
- Keep Cargo environment normalization in `.cargo/config.toml`; `Justfile` Cargo recipes should call `cargo` directly instead of repeating shell-specific `env -u` wrappers.

## Validation

Run before handoff:

```sh
just check
just test
```

Run Pi admin/Caddy smoke validation when deployment files or admin routes change:

```sh
just run-pi-admin
just validate-pi-admin-caddy
```

Latest local validation on 2026-06-23:

- `just check`
- `just test`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `just validate-pi-admin-caddy`

Admin UI validation:

- `just build-admin-ui`
- `just check-admin-ui`

Latest admin UI styling validation on 2026-06-23:

- `just build-admin-ui`
- `just check-admin-ui`
- `just check`
- `just test`
- `caddy validate --config deploy/pi-admin-caddy/Caddyfile`
- Direct Rust admin smoke at `http://127.0.0.1:8080/`: HTML shell, hashed CSS/JS assets, `/api/pi/v1/status`, `/api/auth/session`, `/api/setup/review`, rendered Svelte dashboard, five button cards from real setup data, and mobile viewport overflow check.

Content endpoint smoke note:

- Direct unauthenticated active/inactive content requests returned `{"detail":"authentication required"}`, matching the protected API behavior. Browser login/bootstrap surfaces render, but a credentialed content-management browser flow was not performed against the local data store.

Latest CI workflow validation on 2026-06-23:

- `.github/workflows/ci.yml` parses as YAML locally.
- `env CI=true pnpm --dir admin-ui install --frozen-lockfile`
- `just check`
- `just test`
- `just check-admin-ui`
- `just build-admin-ui`
