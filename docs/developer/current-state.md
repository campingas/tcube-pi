# Current Project State

Last updated: 2026-06-30 (+07)

## Current Focus

- `tcube-pi` is extracted as a standalone Rust repository for the Raspberry Pi side of T-Cube.
- The root package is `tcube-pi`, the Rust crate import path is `tcube_pi`, and the maintained binaries are `tcube-pi`, `tcube-pi-admin`, `tcube-pi-admin-measure`, and `tcube-pi-device-api`.
- The repository owns the child-facing runtime, keyboard simulator, local SQLite state, Pi-hosted admin API, checked-in static admin UI, content sync client/API compatibility code, Caddy deployment files, default content assets, and Pi hardware smoke payloads.
- `docs/developer/FEATURES.md` documents the currently implemented Rust features and the admin UI capabilities.
- The Mac-hosted TTS workers are intentionally not part of this repository. The checked-in `admin-ui/build/` is the static build output from the Svelte + Vite source in `admin-ui/` and is served by `tcube-pi-admin`; generated speech may call external HTTPS speech services for drafts.
- The admin UI has been rebuilt as a dark-mode-only, mobile-first parent dashboard aligned with `VISION.md` and `docs/developer/branding-guide.md`: compact top bar, horizontal status strip, cube hero, button strip, quick actions, recent activity, setup checklist, full-screen button configuration drilldown, active/draft content review, record/upload/generate draft flows, owner tools, and local runtime boundary copy.
- The Pi admin HTTPS boundary remains Caddy in front of loopback-only `tcube-pi-admin`: Rust listens on `127.0.0.1:8080`, and Caddy serves `https://tcube.local/` and `https://10.55.0.1/` with `tls internal`.

## Implemented

- Keyboard simulator for button development without Raspberry Pi hardware.
- Local audio playback through approved content assets.
- SQLite event logging and setup/debug event separation.
- Pi-hosted Rust admin API with setup, authentication, content, media, generated speech, recent button events, and local static/media serving endpoints.
- Versioned admin API aliases under `/api/pi/v1/*` are supported for auth, setup, content, media, and recent events while legacy unversioned paths remain available.
- Language button content listing falls back from the selected language to existing active/draft content attached to the same button, so admin review does not appear empty after a button language change.
- Parent-created audio drafts are stored under `data/audio/draft/{content_type}/` and activation moves them to `data/audio/active/{content_type}/`; legacy `data/media/...` preview paths remain readable for existing rows.
- Svelte + Vite + Tailwind admin UI source under `admin-ui/`, with checked-in static build output under `admin-ui/build/`.
- Local account, session, recovery-code, and manager-invitation primitives.
- Content sync API/client modules for staged package download, checksums, activation, acknowledgements, and failure reporting.
- Caddy deployment sketch and one-button Pi Zero bench smoke payload.
- Rust formatting, linting, and test workflows through `just`.
- GitHub Actions CI under `.github/workflows/ci.yml` runs Rust formatting, check, Clippy, tests, and admin UI pnpm install/check/build gates.
- GitHub Actions release workflow under `.github/workflows/release.yml` builds Linux arm64 Pi Zero 2 W application bundles with Rust binaries, prebuilt admin UI, content, Caddy/systemd deployment files, installer, and SHA-256 checksums.
- Release preparation uses `just prepare-release vX.Y.Z` to update `Cargo.toml`, `Cargo.lock`, and `admin-ui/package.json` before committing and tagging. The release workflow verifies the tag version matches both manifests before publishing.
- Version `0.0.2` is prepared as the first stable release candidate after prerelease validation; push annotated tag `v0.0.2` to publish the GitHub Release bundle.
- Repository Rust workflow config uses `rust-toolchain.toml` for stable Rust with `rustfmt` and `clippy`, plus `.cargo/config.toml` to force inherited host C/C++ flags empty for native dependency consistency.

## Not Yet Complete

- Real Raspberry Pi GPIO backend validation.
- Final LED output backend.
- MAX98357A I2S audio validation from the Rust runtime on target hardware.
- Mini USB microphone capture, retention, upload, and physical indicator privacy rules.
- Installed Pi systemd validation for the final runtime.
- USB OTG recovery and Wi-Fi rollback behavior.
- Pi resource measurements with `just measure-pi-admin`.
- Full flashable SD-card image artifacts.

## Known Issues

- Admin API storage boundaries are now split across focused `src/db/admin/` modules; remaining `content_items` SQL in `src/server/handler.rs` is limited to test fixtures and assertions.

## Architectural Decisions

- Keep child-facing behavior local, deterministic, and independent of network or AI availability.
- Keep `tcube-pi` Rust binaries as thin entry points backed by reusable library modules under `src/`.
- Keep Caddy as the HTTPS browser boundary rather than terminating TLS inside the Rust admin service.
- Keep the admin UI as same-origin static assets. API calls use relative paths so the dashboard works behind Caddy at `https://tcube.local/` without a hardcoded backend URL.
- Use `pnpm` for every admin UI and JavaScript workflow; no Node runtime is required on the Pi.
- Keep admin UI API contracts in `admin-ui/src/api.ts` and inspect Rust handler tests before adding or changing endpoint request bodies. Do not guess backend fields from UI needs.
- Keep admin UI flows mobile-first. Desktop and tablet layouts may add width and columns, but setup, button configuration, content review, and owner tools must remain complete from a phone viewport.
- Report setup dashboard URLs as standard HTTPS URLs without an explicit port, matching the Caddy deployment on port 443.
- Keep Mac-hosted speech and AI workers outside this repository; generated speech is a draft artifact until reviewed and activated locally.
- Keep parent-created audio files under the admin media root, with inactive drafts separated from active child-playable content on disk.
- Keep `just` as the only documented command runner.
- Keep release-bundle helper scripts in `deploy/pi-release/`. Keep runtime Caddy, systemd, and environment files in `deploy/pi-admin-caddy/`.
- Keep Cargo environment normalization in `.cargo/config.toml`; `Justfile` Cargo recipes should call `cargo` directly instead of repeating shell-specific `env -u` wrappers.
- Prepare release version bumps before tag creation; release workflows should enforce manifest/tag consistency rather than mutating manifests after a tag is pushed.

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

Latest admin UI rebuild validation on 2026-06-28:

- `just check-admin-ui`
- `just build-admin-ui`
- `just check`
- `just test`
- Static admin UI build output regenerated under `admin-ui/build/`.
- Added authenticated `/api/pi/v1/events/recent` support for the dashboard activity feed.

Latest mobile-first admin UI draft replication on 2026-06-28:

- `just check-admin-ui`
- `just build-admin-ui`
- Main dashboard now follows `tcube_admin_main_dashboard.html` as a working Svelte view with live status, cube, buttons, setup, quick-action, and activity data.
- Button configuration now follows `tcube_admin_mobile_button_config.html` as a full-screen Svelte drilldown opened from each button card.
- Lucide icons are rendered through `@lucide/svelte`; the static draft's Tabler icon classes are not carried into the app.

Latest content visibility API validation on 2026-06-28:

- `cargo fmt --all --check`
- `cargo test content_lifecycle_lists_activates_trashes_and_cleans_generated_drafts --all-features`
- `just check-admin-ui`
- `just check`
- `just test`
- Local data inspection showed user `campingas` had Button 1 configured as French while existing active language content was English; content APIs now keep same-button content visible in that drift state.

Latest credentialed admin UI browser smoke on 2026-06-28:

- Ran `tcube-pi-admin` on `127.0.0.1:18080` against a temporary copy of `data/tcube.sqlite3`.
- Seeded a temporary `tcube_session` for local user `campingas` in the copied database only.
- Used Playwright CLI to load the built admin UI, verify the authenticated dashboard shows `Signed in as campingas`, confirm the mobile dashboard shows `30 Active sounds`, and confirm the three content buttons show `10 sounds` each.
- Clicked the Top button card and verified the button configuration screen shows `Top · Language`, `Active 10`, active language rows, and `Record` as the selected Add content tab.

Latest recording draft flow validation on 2026-06-28:

- `cargo fmt --all --check`
- `cargo test multipart_recording_and_upload_create_inactive_drafts --all-features`
- `just check-admin-ui`
- `just build-admin-ui`
- `just check`
- `just test`
- Language recordings, uploads, and generated speech require spoken text but do not require a parent-entered title; the stored title/filename is generated from source or TTS model, language, spoken text, and timestamp. Generated speech titles use `generated-{tts-model}-{language}-{text}-{timestamp}`.
- Animal and music recordings/uploads require only a title. Generate is available only for language buttons.
- The button configuration sticky footer now submits the pending Add content action when a recording/upload/generated speech draft is ready; otherwise it saves the button mode.

Latest audio draft storage validation on 2026-06-29:

- `cargo test multipart_recording_and_upload_create_inactive_drafts --all-features`
- `cargo test content_lifecycle_lists_activates_trashes_and_cleans_generated_drafts --all-features`
- `just check`
- `just test`
- `just run-pi-admin` now points `--media-root` at `data/audio`.
- Recorded, uploaded, and generated parent-created media use `data/audio/draft/...` database paths and `/api/media/draft/...` preview URLs.
- Activating a draft moves the file from `data/audio/draft/...` to `data/audio/active/...` and updates the content row before returning the active preview URL.

Latest Pi admin phone-access helper on 2026-06-29:

- `just run-pi-admin` prints the loopback backend URL, Caddy phone/laptop URL `https://tcube.local/`, USB gadget URL `https://10.55.0.1/`, and a detected host LAN address before launching the Rust admin service.
- `just run-pi-admin-caddy` starts Caddy from a temporary Caddyfile-adapted config that includes the detected LAN IP as an HTTPS site name for same-network phone testing.
- Phone access remains through Caddy HTTPS on the same network; the Rust backend stays loopback-only on `127.0.0.1:8080`.

Latest draft audio cleanup fix on 2026-06-29:

- Rejecting an inactive recorded, uploaded, or generated draft now deletes its `data/audio/draft/...` media file before marking the content row as trash.
- Clearing generated speech drafts for a language button now trashes every inactive recorded, uploaded, and generated language draft for that button/language and deletes matching `data/audio/draft/...` files.

Latest button active-count fix on 2026-06-29:

- Button cards and the button configuration header now display the loaded per-button active content count instead of taking the maximum of that list count and the setup-level content-type count.
- Summary count render expressions now pass `contentState` explicitly into active/draft count helpers so button strip and selected-button stats update after content activation refreshes.
- Button mode changes can reuse existing modes/languages on multiple buttons, the footer `Save mode` action remains enabled without requiring pending media, and active/draft content lists stay scoped to the selected button/language instead of falling back to other content.

Latest content empty-state clarity on 2026-06-29:

- Active and draft content list endpoints now return `{ items, empty_state }` so empty button scopes can explain whether no content exists yet, the same button has content in another language, or another button has content for the selected mode/language.
- The admin UI renders backend-provided empty-state titles/details in the button configuration Active and Drafts tabs while continuing to count and display content from `items`.
- Content listing API failures now render a visible inline error directly below the Active/Drafts tabs in the button configuration screen.

Latest content inventory view on 2026-06-29:

- Added authenticated `/api/pi/v1/content/inventory` support for all active and draft audio rows with current-setup classification as `active`, `draft`, or `unused`.
- The dashboard now links to a content inventory view grouped by unused audio, draft audio, and active audio, with per-row button, mode/language, source, reason, preview audio, and open-button action.

Latest mobile viewport Playwright coverage on 2026-06-29:

- Added `admin-ui/playwright.config.ts` with a mobile Chromium project, fixed viewport, and an on-demand dev-server launch for repeatable mobile checks.
- Added `just test-admin-ui-mobile` as the documented entrypoint for the mobile viewport checks.
- The mobile suite now covers dashboard stacking, button selector sizing, and button-config icon updates, and it runs against mocked Pi API responses so the checks stay local and fast.
- Removed snippet-unsafe dynamic icon rendering from the admin UI status bar and button surfaces after Svelte runtime errors surfaced under Playwright.
- The T-Cube logo now acts as a home control in the authenticated top bar and returns the dashboard view.
- The signed-in identity line now splits username and role into separate top-bar spans, with distinct colors for the username and for owner/admin/member.
- Latest validation after the fix: `just check-admin-ui`, `just build-admin-ui`, and `just test-admin-ui-mobile`.

Latest top-bar status update on 2026-06-29:

- The status chip next to Wi-Fi now reads `LLMs offline` with a warning icon and warning color instead of `Admin`.
- The top-bar status strip now folds away the all-good chips and leaves only not-ready warning items visible.
- The authenticated top bar no longer shows a refresh button; dashboard refresh still exists on the page where needed.

Latest active-list row update on 2026-06-29:

- Button-configuration active rows now show a trimmed audio filename as the primary line, with `Default/Recorded/Uploaded/Generated · duration · x plays` underneath.
- Active rows no longer render inline audio controls; the full row is clickable for preview playback and the trash action opens a custom confirmation dialog.
- The play-count text is still a placeholder because the backend does not track per-audio plays yet.
- Mobile Playwright coverage now verifies the trimmed title, summary line, row click target, and custom trash modal.

Latest recording feedback update on 2026-06-29:

- The button-configuration recording flow now shows explicit idle, recording, processing, ready, and saving guidance so parents can tell when capture starts, stops, and becomes a draft-ready preview.
- While recording, the admin UI uses the browser Web Audio analyser on the active microphone stream to render a live microphone-level waveform.
- Stopping, switching away from Record, discarding, saving, or leaving the app cleans up the microphone stream, recorder timer, waveform animation, and audio context.
- Mobile Playwright coverage now mocks browser recording APIs and verifies the idle hint, live waveform, ready preview state, and language text requirement before saving.

Latest generated TTS offline handling on 2026-06-29:

- Added authenticated generated-speech provider status support so the admin UI can detect local TTS availability without submitting a generation request.
- Provider status checks are cached by resolved provider/base URL, and the Generate tab rechecks offline providers with backoff instead of repeatedly calling the Mac TTS service.
- When TTS is offline, the Generate add-content flow shows an inline warning and disables generated-speech controls until a later health check reports online; Record, Upload, mode changes, and existing content review remain usable.
- The main dashboard status strip uses the same generated-speech status API to switch the LLMs chip between offline and online without a tight polling loop.
- Mobile Playwright coverage now verifies the offline notice, disabled Generate controls, unaffected Record controls, and recovery after a later online status response.

Latest settings page implementation on 2026-06-29:

- The top-right Settings action now opens a mockup-aligned grouped settings screen with Cube, Account, Manager invitations, Danger zone, logout, and version/status footer sections.
- Settings rows are wired to existing APIs for cube name, Wi-Fi verification, recovery code creation, manager invitation creation, clipboard copy, and logout.
- Manager invitation copy now copies the full browser URL with `?invite=...`, matching the accept-invitation flow instead of copying only the raw code.
- The authenticated top bar renders manager accounts as `manager` while keeping the existing manager role color styling.
- Added owner-only `DELETE /api/pi/v1/content/unused`, which reuses content inventory classification to trash active audio that is unused by the current button setup and delete matching `data/audio/...` files.
- Added owner-only `POST /api/pi/v1/setup/factory-reset`, wired to the Settings Danger zone with typed `FACTORY RESET` confirmation, to clear setup, accounts, sessions, content rows, events, sync state, and parent-created `data/audio/{draft,active}/` media before reseeding defaults.
- Password change and session revocation remain visually present but disabled because the local API contracts do not exist yet.
- Mobile Playwright coverage now verifies settings layout, save-name, recovery-code, manager-invite, clear-unused-content, factory-reset confirmation, and viewport overflow behavior.
- The unauthenticated Create local owner page no longer shows a refresh action, and the dashboard Setup incomplete checklist now appears directly below the refreshed-state notice; the Wi-Fi prerequisite opens Settings with Wi-Fi verification expanded.
- First owner bootstrap now provisions or reuses a local cube identity and creates an owner membership immediately, including after factory reset, so the top bar reports `owner` and owner-only setup actions remain available before the cube name is changed. Existing single-account databases missing that membership self-repair on session read or password login.

Latest admin server boundary refactor on 2026-06-29:

- `tcube-pi-admin` now registers explicit Axum routes for the versioned and legacy admin API paths instead of serving the API through the catch-all fallback dispatcher.
- Admin SQLite opening, WAL setup, migrations, default seed data, permission tightening, and table-inspection helpers now live under `src/db/admin/schema.rs`; `server::run` initializes the database before binding the listener.
- Admin account lookup, password hashing/verification, session creation/authentication/revocation, invitation membership helpers, local cube role checks, and first-owner membership repair now live under `src/db/admin/auth.rs`.
- Setup review reads, cube naming, Wi-Fi verification, button mode persistence, setup completion, and factory reset database clearing/reseeding now live under `src/db/admin/setup.rs`.
- Content item insertion, active/draft listing, empty-state detection, inventory classification, activation, trashing, media artifact rows, and content order allocation now live under `src/db/admin/content.rs`.
- Added Axum service coverage for the production route table with versioned status and owner bootstrap requests.
- Latest validation after the refactor: `cargo fmt --all --check`, `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `just check`, and `just test`.

Latest CI workflow validation on 2026-06-23:

- `.github/workflows/ci.yml` parses as YAML locally.
- `env CI=true pnpm --dir admin-ui install --frozen-lockfile`
- `just check`
- `just test`
- `just check-admin-ui`
- `just build-admin-ui`

Latest release workflow validation on 2026-06-23:

- `.github/workflows/release.yml` parses as YAML locally.
- `bash -n deploy/pi-release/install-on-pi`
- `bash -n deploy/pi-release/prepare-release`
- `just check`
- `just test`
- `just check-admin-ui`
- `just build-admin-ui`
- `cargo build --release --locked --all-features`

Latest release preparation validation on 2026-06-23:

- `just prepare-release v0.0.2`
- `just check`
- `just check-admin-ui`
- `just test`
- `just build-admin-ui`
- `cargo build --release --locked --all-features`
