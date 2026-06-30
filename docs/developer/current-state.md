# Current Project State

Last updated: 2026-07-01 (+07)

## Current Focus

- `tcube-pi` is extracted as a standalone Rust repository for the Raspberry Pi side of T-Cube.
- The root package is `tcube-pi`, the Rust crate import path is `tcube_pi`, and the maintained binaries are `tcube-pi`, `tcube-pi-admin`, and `tcube-pi-admin-measure`.
- The repository owns the child-facing runtime, keyboard simulator, local SQLite state, Pi-hosted admin API, checked-in static admin UI, Caddy deployment files, default content assets, and Pi hardware smoke payloads.
- `docs/developer/FEATURES.md` documents the currently implemented Rust features and the admin UI capabilities.
- The Mac-hosted TTS workers are intentionally not part of this repository. The checked-in `admin-ui/build/` is the static build output from the Svelte + Vite source in `admin-ui/` and is served by `tcube-pi-admin`; generated speech may call external HTTPS speech services for drafts.
- The admin UI has been rebuilt as a dark-mode-only, mobile-first parent dashboard aligned with `VISION.md` and `docs/developer/branding-guide.md`: compact top bar, horizontal status strip, cube hero with Wi-Fi and USB detail lines, button strip, quick actions, recent activity, setup checklist, full-screen button configuration drilldown, active/draft content review, record/upload/generate draft flows, owner tools, and local runtime boundary copy.
- The Pi admin HTTPS boundary remains Caddy in front of loopback-only `tcube-pi-admin`: Rust listens on `127.0.0.1:8080`, and Caddy serves `https://tcube.local/` and `https://10.55.0.1/` with `tls internal`.

## Implemented

- Keyboard simulator for button development without Raspberry Pi hardware.
- Local audio playback through approved content assets.
- SQLite event logging and setup/debug event separation.
- Pi-hosted Rust admin API with setup, authentication, content, media, generated speech, recent button events, and local static/media serving endpoints.
- Versioned admin API aliases under `/api/pi/v1/*` are supported for auth, setup, content, media, and recent events while legacy unversioned paths remain available.
- The recent activity API now returns a unified feed from runtime button events plus admin activity events for successful sign-in, recording, upload, generated speech, activation, and delete/trash operations.
- Language button content listing falls back from the selected language to existing active/draft content attached to the same button, so admin review does not appear empty after a button language change.
- Parent-created audio drafts are stored under `data/audio/draft/{content_type}/` and activation moves them to `data/audio/active/{content_type}/`; legacy `data/media/...` preview paths remain readable for existing rows.
- Svelte + Vite + Tailwind admin UI source under `admin-ui/`, with checked-in static build output under `admin-ui/build/`.
- Local account, session, recovery-code, and manager-invitation primitives.
- The cube hero now renders a verified Wi-Fi SSID and dashboard IP on separate lines, uses a default `wi-fi · 192.168.0.1` fallback before verification, and shows an explicit USB connection state from the admin status payload.
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

Latest admin UI hero/status refresh on 2026-07-01:

- `just build-admin-ui`
- `just check-admin-ui`
- `just test-admin-ui-unit`
- `just test-admin-ui-mobile`
- `just check`
- `just test`
- The dashboard hero now shows separate Wi-Fi and USB subtitle lines, removes the old reachability badge and dot, and uses icon color to reflect verified Wi-Fi and USB connection state.
- The setup review payload now includes `wifi_ssid`, and the admin status payload now includes an explicit `usb_connected` flag.
- The buttons section action is now icon-only, with the visible `Manage all` label removed.

Latest recent activity feed update on 2026-07-01:

- Added `admin_activity_events` to SQLite so successful admin sign-in and content mutations are recorded separately from runtime-owned `button_events`.
- `/api/pi/v1/events/recent` now returns normalized mixed activity rows with event kind, button label, content metadata, audio filename, and elapsed-time-ready timestamps.
- The dashboard Recent activity card now renders dedicated icons, action text, and relative time for sign-in, button press, recorded, uploaded, generated, activated, and deleted activity.
- Validation: `just check`, `just test`, `just check-admin-ui`, `just build-admin-ui`, `just test-admin-ui-unit`, and `just test-admin-ui-mobile`.

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

Latest admin route extraction on 2026-06-30:

- Shared HTTP request/response primitives now live in `src/server/http.rs`, auth endpoint handlers live in `src/server/routes/auth.rs`, setup endpoint handlers live in `src/server/routes/setup.rs`, and content management endpoint handlers live in `src/server/routes/content.rs`.
- Local media/file helpers now live in `src/server/media.rs`, including multipart parsing, WAV inspection/validation, media filename/path construction, preview URL construction, draft writes, draft-to-active movement, and audio deletion helpers.
- Speech provider generation, health caching, HTTPS URL validation, and blocking HTTP client construction now live in `src/server/speech.rs`.
- `src/server/handler.rs` remains responsible for legacy handler dispatch plus status, media creation endpoint orchestration, generated speech endpoint orchestration, static assets, and recent-event behavior while routes continue moving behind Axum handlers.
- Route tests for the extracted admin endpoints use Axum `oneshot` coverage instead of the deleted test-only dispatcher.

Latest native Axum route conversion on 2026-06-30:

- Admin API routes now use native Axum extractors for state, JSON bodies, path params, query params, multipart uploads, and `tcube_session` cookies through a small route-layer extractor.
- Route errors now flow through `src/server/routes/error.rs` as JSON `{ "detail": ... }` responses while preserving the existing 400, 401, and 500 status mappings.
- Auth, setup, content management, generated speech, recent events, recording upload, and file upload handlers now receive typed inputs instead of `HttpRequest`.
- Axum multipart support is enabled through the existing Axum dependency, replacing the custom multipart parser while preserving draft creation and validation behavior.

Latest legacy admin HTTP adapter removal on 2026-06-30:

- Static admin UI, admin media, and bundled content routes now return native Axum `Response` values with preserved MIME types, content lengths, SPA fallback behavior, path traversal rejection, and JSON error bodies.
- The admin server no longer has `src/server/http.rs`; `HttpRequest`, `HttpResponse`, `json_response`, `error_response`, `run_request`, and versioned-path canonicalization were removed from the admin route layer.
- Admin route tests use a local test request builder for Axum `oneshot` coverage.

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

Latest device sync removal on 2026-06-30:

- Removed the unused `tcube-pi-device-api` binary and the `device_api`/`device_sync` library modules because they are not wired into runtime playback, the Pi admin service, the admin UI, release update handling, or child-facing behavior.
- Release packaging and the Pi installer now ship only the maintained Pi binaries: `tcube-pi`, `tcube-pi-admin`, and `tcube-pi-admin-measure`.
- Product and developer docs now treat external content sync as deferred future discovery work until the parent/device use case, update source, package format, auth model, rollback behavior, and privacy rules are defined.
- Existing SQLite content package and failure tables remain in place for now; schema cleanup should be handled as a separate migration decision.
- Validation after removal: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `just check`, and `just test`.

Latest Rust admin optimization pass on 2026-06-30:

- Content inventory classification now loads button mappings once per request instead of re-querying mappings for every inventory row.
- Authenticated session reads now roll `last_seen_at` and `expires_at` at most once every 10 minutes per active session, while expired, revoked, and disabled-account sessions are still rejected on every request.
- Static admin UI, media, and bundled content responses now use Tower static file services instead of blocking whole-file reads in async handlers, while preserving existing routes and traversal rejection.
- Multipart audio uploads now read the audio field in chunks and reject audio larger than 25 MB during field parsing; the route body limit was raised enough to allow valid 25 MB audio plus multipart overhead.
- Admin schema migration now creates targeted `content_items` indexes for list, draft cleanup, and inventory query paths.
- Validation after the optimization pass: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `just check`, and `just test`.

Latest native-only development cleanup on 2026-06-30:

- Removed the unused alternate development artifacts and recipe. Local development and Pi validation now use native host/Pi commands through `just`.
- Updated contributor, package, and task docs to remove the obsolete alternate development/runtime path.

Latest admin server boundary refactor on 2026-06-29:

- `tcube-pi-admin` now registers explicit Axum routes for the versioned and legacy admin API paths instead of serving the API through the catch-all fallback dispatcher.
- Admin SQLite opening, WAL setup, migrations, default seed data, permission tightening, and table-inspection helpers now live under `src/db/admin/schema.rs`; `server::run` initializes the database before binding the listener.
- Admin account lookup, password hashing/verification, session creation/authentication/revocation, invitation membership helpers, local cube role checks, and first-owner membership repair now live under `src/db/admin/auth.rs`.
- Setup review reads, cube naming, Wi-Fi verification, button mode persistence, setup completion, and factory reset database clearing/reseeding now live under `src/db/admin/setup.rs`.
- Content item insertion, active/draft listing, empty-state detection, inventory classification, activation, trashing, media artifact rows, and content order allocation now live under `src/db/admin/content.rs`.
- Added Axum service coverage for the production route table with versioned status and owner bootstrap requests.
- Latest validation after the refactor: `cargo fmt --all --check`, `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `just check`, and `just test`.

Latest admin route test conversion on 2026-06-30:

- Admin route tests now drive the production Axum router with `tower::ServiceExt::oneshot` instead of the legacy test-only `route_request` dispatcher.
- The test-only dispatcher in `src/server/routes/mod.rs` has been removed; production versioned-path canonicalization remains in the shared route request wrapper.
- Existing auth, setup, content, static-content, multipart upload, factory reset, manager-permission, and versioned-alias assertions were preserved while switching their HTTP path through Axum.
- Latest validation after the conversion: `cargo test --workspace --all-features`.

Superseded HTTP primitive extraction on 2026-06-30:

- Shared admin HTTP primitives briefly lived in `src/server/http.rs` during route extraction.
- This intermediate adapter was removed by the native Axum cleanup; admin routes and static/media responses now use Axum extractors and responses directly.

Latest auth route extraction on 2026-06-30:

- Admin auth endpoint handlers and DTOs now live in `src/server/routes/auth.rs`: session read, password login, first-owner bootstrap, password recovery, recovery code creation, manager invitations, invitation acceptance, logout, and auth cookie helpers.
- `src/server/handler.rs` no longer owns auth endpoint business logic; media creation, speech provider, status, and recent-event handlers remain there pending later extraction.

Latest task backlog cleanup on 2026-06-30:

- Reviewed `src/` and `admin-ui/src/` for next implementation gaps, stale TODO-style work, maintainability hotspots, and product-risk ordering.
- Replaced `docs/tasks.md` with a focused, priority-ordered next-work backlog and moved completed-history expectations to this current-state document.
- Highest-priority remaining work is target Raspberry Pi validation, real GPIO input, I2S output validation, admin-load latency measurement, LED output, and microphone privacy rules before capture implementation.
- Backend cleanup targets include hardening malformed WAV validation, adding real per-content play counts, formalizing SQLite migrations, and finishing the migration away from `src/server/handler.rs` toward typed Axum route modules.
- Admin UI cleanup targets include splitting the large `admin-ui/src/App.svelte` into focused views/state components, removing stale unused component prototypes, and preserving mobile Playwright coverage during extraction.

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

Latest audio validation hardening on 2026-06-30:

- WAV inspection now rejects short or truncated `fmt ` chunks with a clean `recorded WAV file is malformed` error instead of panicking on unchecked slice reads.
- WAV parsing now uses checked little-endian field reads for chunk sizes, `fmt ` fields, and sample data.
- Added parser coverage for valid PCM WAVs, short `fmt ` chunks, truncated chunks, missing data chunks, non-PCM audio, non-16-bit audio, quiet audio, and over-duration language audio.
- Multipart recording and upload coverage now verifies malformed WAV files return HTTP 400 JSON errors while preserving valid draft creation and 25 MB rejection coverage.
- Validation after the hardening: `cargo test inspect_wav --all-features`, `cargo test validate_wav --all-features`, `cargo test multipart_recording_and_upload_create_inactive_drafts --all-features`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features`.

Latest per-content play-count update on 2026-06-30:

- Active content list responses now include `play_count`, computed from local `button_events.response_id` matches against each `content_items.id`.
- Admin schema migration now creates `idx_button_events_response_id` to keep play-count aggregation inexpensive.
- Button-configuration active rows now render real all-time counts as `0 plays`, `1 play`, or `{n} plays` instead of the previous `x plays` placeholder.
- Backend coverage verifies nonzero and zero active-item play counts, and mobile Playwright coverage verifies real play-count text in active rows.
- Validation after the update: `cargo test content_lifecycle_lists_activates_trashes_and_cleans_generated_drafts --all-features`, `just check-admin-ui`, `just build-admin-ui`, `just test-admin-ui-mobile`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `just check`, and `just test`.

Latest admin route extraction completion on 2026-06-30:

- Multipart recording/upload orchestration and generated-speech draft/status orchestration now live in `src/server/routes/content.rs`.
- Admin status response construction now lives in `src/server/routes/status.rs`, and recent button-event reads now live in `src/server/routes/events.rs`.
- Production server code no longer imports or calls `src/server/handler.rs`; that module is compiled only for legacy route test coverage.
- Validation after the extraction: `cargo test multipart_recording_and_upload_create_inactive_drafts --all-features`, `cargo test generated_language_filename_includes_model_language_and_text --all-features`, `cargo test versioned_admin_api_aliases_support_session_setup_and_events --all-features`, `cargo test content_lifecycle_lists_activates_trashes_and_cleans_generated_drafts --all-features`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `just check`, and `just test`.

Latest admin UI route/component split on 2026-06-30:

- `admin-ui/src/App.svelte` now delegates authenticated dashboard, button configuration, inventory, settings, and auth flows to focused typed view components instead of a single inline render tree.
- Shared content and status presentation moved into `admin-ui/src/components/` and `admin-ui/src/view-utils.ts`, and stale prototype components were removed.
- The mobile recording flow keeps the draft-text footer enablement in sync with the live input, and the generated-speech footer no longer duplicates the form submit label.
- Validation after the split: `just check-admin-ui`, `just build-admin-ui`, and `just test-admin-ui-mobile`.

Latest admin UI split follow-up on 2026-06-30:

- Auth submit buttons again respect the shared busy state to prevent duplicate login, bootstrap, recovery, and invitation submissions.
- Button content draft form edits now flow through an explicit `updateDraftForm` patch action owned by `App.svelte`, so generated-speech status checks react to provider/text changes without hidden child prop mutation.
- Button configuration footer label and disabled-state decisions now use named helper functions and the canonical draft form state instead of a local draft-text sync bridge.
- Validation after the follow-up: `just check-admin-ui`, `just build-admin-ui`, and `just test-admin-ui-mobile`.

Latest admin UI controller extraction on 2026-06-30:

- Button configuration view-model helpers, content keys/counts, immutable draft-form patching, and footer action decisions now live in a typed `button-config-controller.ts` module.
- Generated-speech health keys, menu status keys, disabled-state calculation, provider-key parsing, offline status creation, and backoff transitions now live in a typed `generated-speech-health.ts` module.
- Added dependency-free Node TypeScript unit coverage for footer action states, draft-form patching, generated-speech keys, disabled states, offline status, and backoff transitions, with `just test-admin-ui-unit` and a matching CI step.
- Validation after the extraction: `just test-admin-ui-unit`, `just check-admin-ui`, `just build-admin-ui`, and `just test-admin-ui-mobile`.

Latest admin UI recording controller extraction on 2026-06-30:

- Recording/upload decision logic now lives in a typed `recording-controller.ts` module: upload type/size validation, recording status transitions, recording guidance copy, media draft validation, waveform level normalization, default waveform state, and generated draft title fallback.
- Browser-owned objects remain in `App.svelte` for now: `MediaRecorder`, `AudioContext`, timers, object URLs, and stream cleanup still stay next to the lifecycle code that creates them.
- Node unit coverage now verifies upload limits, recording status transitions, recording/save guidance, language versus music draft requirements, default titles, and waveform normalization.
- Validation after the extraction: `just test-admin-ui-unit`, `just check-admin-ui`, `just build-admin-ui`, and `just test-admin-ui-mobile`.

Latest localhost admin UI fix on 2026-06-30:

- Static SPA fallback responses now serve `index.html` as `text/html` with `Content-Disposition: inline`, so opening `https://localhost/` renders the 449-byte admin HTML shell instead of downloading it.
- Static fallback MIME selection now uses the normalized safe path, so extensionless SPA routes and rejected static paths also fall back to inline HTML consistently.
- Added regression coverage for root fallback, extensionless fallback, rejected-path fallback, and the real Axum router `/` fallback.
- HTTPS Caddy smoke on `https://localhost:18443/` returned `content-type: text/html; charset=utf-8`, `content-disposition: inline`, and `content-length: 449`.
- Validation after the fix: `cargo test serves_html_for --all-features`, `cargo test router_serves_admin_index_as_inline_html --all-features`, `cargo fmt --all --check`, `just check`, and `just test`.

Latest dashboard audio drilldown update on 2026-06-30:

- The dashboard no longer shows the duplicate `Content inventory` card or `View all` link; active, draft, and unused counts remain visible in the cube hero stats.
- The cube hero stats are now clickable: `Presses today` opens today's recent play events, while `Active sounds`, `Drafts`, and `Unused` open filtered audio lists from the existing content inventory API.
- Button-configuration active rows and dashboard audio detail rows now share `AudioContentRow`, keeping future audio-row styling changes centralized.
- Mobile Playwright coverage now verifies the removed inventory card, filtered stat detail navigation, and the preserved button-configuration active-row behavior.
- Validation after the update: `just check-admin-ui`, `just test-admin-ui-unit`, `just test-admin-ui-mobile`, and `just build-admin-ui`.

Latest Add content UI restoration on 2026-07-01:

- Button configuration `Add content` now uses the richer mobile-first composer again: icon tabs, large record/stop control, waveform/status panel, framed upload zone, and structured generated-speech fields.
- Existing recording, upload, generated-speech health, draft validation, and sticky footer behavior were preserved.
- Mobile Playwright coverage now checks the restored Add-content tab styling, record control, upload zone, and recording save state.
