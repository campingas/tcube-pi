# Tasks

Reviewed: 2026-07-01 (+07)

This file tracks only active next work, ordered by product impact and implementation risk. Completed work belongs in git history; keep only live status in `docs/developer/current-state.md`.

## Next

- [ ] Validate the current runtime and admin service on Raspberry Pi Zero 2 W target hardware, including install bundle, Caddy HTTPS access, same-origin admin UI, active content preview URLs under `/api/pi/v1/`, and boot-time service behavior.
- [ ] Implement the real Raspberry Pi GPIO input backend behind a hardware feature gate, including debounce behavior, one physical MakerEdu MKE-M02 button smoke validation, and simulator-safe desktop tests.
- [ ] Validate MAX98357A I2S audio output from the Rust runtime on the Pi Zero 2 W, then run `just measure-pi-admin` with the admin service under load to confirm button-to-audio latency remains within the product target.
- [ ] Implement LED output for the five RGB buttons, including normal button feedback, inactive/disabled feedback, setup/error feedback, and the mandatory microphone-active indicator before any mic capture work ships.
- [ ] Wire Learning stats and Run curation only after local API contracts exist, keeping disabled UI actions visually distinct until the backend is real.
- [ ] Define microphone capture privacy rules before implementation: capture lifetime, retention, deletion, upload boundaries, Mac-only processing, physical indicator behavior, and parent consent.
- [ ] Implement and validate mini USB microphone capture through the Pi Zero 2 W OTG port only after the privacy and LED indicator rules are documented.
- [ ] Define durable SQLite schema versioning and migrations beyond the current create-if-missing schema path, including upgrade tests for existing local databases.
- [ ] Add acknowledged upload and retention handling for cube event data, content media, and any future microphone-derived data.
- [ ] Clarify the future sync/content package use case: publisher, hosting location, auth, signing, rollback, privacy boundaries, and whether GitHub Releases, Mac-local transfer, parent-device transfer, or cloud sync is responsible.
- [ ] Prepare full flashable SD-card image artifacts after hardware, Caddy, systemd, GPIO, I2S, and resource validation are complete.

## Orchestration (proposed)

Pi-side implementation of `docs/developer/orchestration-blueprint.md`. The Mac stack ships in `deploy/mac-hermes/`; these tasks are ordered (1, 2, 3, then 4 and 5 in parallel, then 6, 7) and the feature stays off by default.

- [ ] Add the `orchestration_jobs` and `orchestration_job_artifacts` schema migration (version 6) plus a `src/db/admin/jobs.rs` storage layer with unit tests.
- [ ] Add `OrchestratorConfig` as an `Option` on `AdminConfig`, env-driven and off by default, with the documented commented block in `deploy/pi-admin-caddy/tcube-pi-admin.env` and the `hmac`/tokio-`time` Cargo additions.
- [ ] Add the `src/server/orchestrator/{envelope,signing}` wire-contract module with serde round-trip and HMAC-vector unit tests.
- [ ] Refactor `save_multipart_media` to extract a shared `save_media_draft` helper and add an optional `artifact_id` to `MediaInput`, with zero behavior change.
- [ ] Add session-authenticated job endpoints (`POST/GET /api/pi/v1/jobs`, `GET /api/pi/v1/jobs/{id}`, `GET /api/pi/v1/orch/status`) plus the missing `ApiError` variants.
- [ ] Add machine-token ingest endpoints (`/api/pi/v1/orch/jobs/{id}/artifacts` and `/result`) with an `OrchBearer` extractor, idempotency, and an end-to-end test proving ingested drafts still require parent activation.
- [ ] Add the outbox dispatcher (tokio task in `tcube-pi-admin`, HMAC-signed webhook POSTs, exponential backoff, reaper) with a mock-Hermes integration test and an offline-tolerance test.

## Maintenance

- [ ] Keep `docs/developer/current-state.md` updated after significant implementation or validation work.
- [ ] For Rust changes, run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features` before handoff.
- [ ] For admin UI changes, run `just build-admin-ui`, `just check-admin-ui`, `just test-admin-ui-unit`, and `just test-admin-ui-mobile` before handoff.
