# Tasks

Reviewed: 2026-07-01 (+07)

This file tracks only active next work, ordered by product impact and implementation risk. Completed work belongs in git history; keep only live status in `docs/developer/current-state.md`.

## Next

- [x] Validate the current runtime and admin service on Raspberry Pi Zero 2 W target hardware, including install bundle, Caddy HTTPS access, same-origin admin UI, active content preview URLs under `/api/pi/v1/`, and boot-time service behavior.
- [x] Validate the new `tcube-pi.service` GPIO runtime end to end on target hardware: fresh install, reboot, GPIO17 button press to speaker audio, admin UI remap taking effect live, Pomodoro 1+2+4 chord, and installer idempotence on re-run.
- [x] Validate MAX98357A I2S audio output from the Rust runtime on the Pi Zero 2 W (rodio through the `ALSA_CARD=MAX98357A` default), then run `just measure-pi-admin` with the admin service under load to confirm button-to-audio latency remains within the product target.
- [ ] Implement LED output for the five RGB buttons: normal button feedback, inactive/disabled feedback, and setup/error feedback. Drive an addressable chain over SPI0 (BCM10) or an I2C PWM driver; PWM and PCM pins are claimed by I2S audio (see the GPIO pin budget in `docs/hardware/hardware-assembly.md`). The mandatory microphone-active indicator is satisfied by the reSpeaker XVF3800 firmware mute LED and is no longer part of this task.
- [ ] Wire Learning stats and Run curation only after local API contracts exist, keeping disabled UI actions visually distinct until the backend is real.
- [ ] Define microphone capture privacy rules before implementation: capture lifetime, retention, deletion, upload boundaries, Mac-only processing, physical indicator behavior (the XVF3800 hardware mute button and red mute LED cut audio in firmware before it reaches the Pi), and parent consent.
- [ ] Implement and validate microphone capture with the Seeed reSpeaker XVF3800 USB 4-mic array through the Pi Zero 2 W OTG port (capture-only at 16 kHz; playback stays on the MAX98357A over I2S; board powered from external 5V, not OTG bus power) only after the privacy rules are documented.
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
