# Feature Inventory

This document summarizes what the current repository code can do today, split between the Rust Raspberry Pi runtime/admin backend and the Svelte admin UI.

## Rust Code Features

The Rust code builds three maintained binaries:

- `tcube-pi`: child-facing device runtime and keyboard simulator.
- `tcube-pi-admin`: Pi-hosted local admin API and static admin UI server.
- `tcube-pi-admin-measure`: latency measurement harness for button handling under admin API load.

The child-facing runtime currently supports:

- Loading and validating a local JSON content pack from `content/content.json` or a configured path.
- Five button mappings with language, animal, music, setup-help, and disabled behaviors.
- Deterministic local response selection by mode, cycling through available responses without network or AI dependency.
- A terminal keyboard simulator where keys `1` through `5` trigger cube buttons and `q` or `Esc` exits.
- A simulator TUI that shows button state, dashboard address, recent playback, LED feedback state, and recent activity.
- Local audio playback through `rodio` when launched with the local audio backend.
- Terminal-only audio feedback when local audio playback is not requested.
- Event logging to SQLite, using normal button events after setup completion and setup debug events before setup completion.
- Stubbed Pi GPIO initialization, so physical GPIO behavior still needs implementation and target hardware validation.

The local data and measurement code currently supports:

- SQLite tables for button events, setup debug events, and latency measurements.
- Recording latency measurements for deterministic button selection plus SQLite event logging.
- Querying latency measurements by time range.
- Measuring baseline button latency and button latency while concurrent HTTP requests hit the admin service.
- Reporting p50, p95, p99, max latency, admin request success/failure counts, and load deltas as JSON.

The Pi-hosted admin service currently supports:

- Loopback HTTP service intended to sit behind Caddy HTTPS.
- Static serving for the checked-in admin UI build output.
- Static media serving from configured media and content roots.
- Versioned admin API aliases under `/api/pi/v1/*` for status, auth, setup, content, media, and recent event routes.
- `/api/pi/v1/status` with service, database, UI, media, content, hostname, and USB address facts.
- Local owner bootstrap when no admin account exists.
- Username/password login with scrypt password hashing.
- Hashed random session tokens stored in secure HTTP-only `SameSite=Strict` cookies.
- Session lookup and rolling session cookies.
- Logout and session revocation.
- Recovery code creation and password reset with session revocation.
- Owner-created manager invitation codes and manager account acceptance.
- Role checks for owner-only setup actions and authenticated content actions.
- Cube name provisioning into local SQLite setup/device records.
- Wi-Fi verification state and dashboard IP storage.
- Setup completion with response metadata for LED pattern, spoken confirmation, and dashboard address.
- Per-button mode configuration for five buttons.
- Active content listing by button, content type, and language where relevant.
- Inactive draft content listing.
- Uploading recorded or uploaded media as inactive content for review.
- Generated speech draft creation through configured external speech providers.
- Activation of inactive content items.
- Moving content items to trash.
- Cleanup of unused generated speech for a selected button and language.
- Authenticated recent activity listing from local SQLite button and admin activity logs for parent monitoring.

## Admin UI Features Right Now

The admin UI is a Svelte and Vite dashboard served as same-origin static files by `tcube-pi-admin`.

It currently can:

- Load service status, session state, setup review data, and visible content state from the Pi admin API.
- Load recent signed-in, content, and button-play events from the Pi admin API activity feed.
- Show whether the backend database, UI distribution, dashboard address, and USB address are configured.
- Create the first local owner account when bootstrap is required.
- Log in and log out with local cube accounts.
- Create and display a recovery code for the signed-in account.
- Reset a password with a recovery code.
- Accept manager invitations from an invitation URL or pasted code.
- Show the signed-in account name, username, and cube role.
- Save the cube name.
- Mark Wi-Fi as verified by storing the SSID and dashboard IP.
- Complete setup when signed in as an owner.
- Create manager invitations when signed in as an owner.
- Display setup prerequisites for owner or manager account, cube name, Wi-Fi, and active language, animal, and music content.
- Display all five cube faces as selectable button cards.
- Configure a selected button as language, animals, music, setup help, or disabled.
- Choose a language for language-mode buttons.
- List active content for the selected button and content type.
- List inactive draft content for review.
- Activate inactive draft content.
- Move active or inactive content to trash.
- Clear unused generated speech for the selected button and language.
- Record browser microphone audio in secure contexts or localhost, convert it to WAV, preview it, and submit it as inactive content.
- Upload an MP3 or WAV file as inactive content.
- Request generated speech using auto, local, or hosted provider selection and an optional voice.
- Show success and error messages from API operations.
- Use relative API paths so the dashboard works behind the Caddy HTTPS boundary without a hardcoded backend URL.
- Use the versioned `/api/pi/v1/*` API namespace while the Rust service keeps legacy unversioned compatibility.

Current limitations:

- Physical Raspberry Pi GPIO, LED output, I2S audio, and USB microphone capture still require implementation or target hardware validation.
- The admin UI manages local setup and content workflows, but child-facing playback remains deterministic and local; generated speech is saved as an inactive draft until activation.
- Browser recording depends on browser microphone support and a secure context.
- External content sync is not implemented; it is deferred until the parent/device use case, update source, package format, auth model, rollback behavior, and privacy rules are defined.
