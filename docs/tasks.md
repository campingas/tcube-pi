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

## Maintenance

- [ ] Keep `docs/developer/current-state.md` updated after significant implementation or validation work.
- [ ] For Rust changes, run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features` before handoff.
- [ ] For admin UI changes, run `just build-admin-ui`, `just check-admin-ui`, `just test-admin-ui-unit`, and `just test-admin-ui-mobile` before handoff.
