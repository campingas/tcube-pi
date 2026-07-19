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

## Maintenance

- [ ] Keep `docs/developer/current-state.md` updated after significant implementation or validation work.
- [ ] For Rust changes, run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features` before handoff.
- [ ] For admin UI changes, run `just build-admin-ui`, `just check-admin-ui`, `just test-admin-ui-unit`, and `just test-admin-ui-mobile` before handoff.
