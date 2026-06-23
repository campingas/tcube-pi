# Tasks

## Active

- [x] Keep Pi admin service behind Caddy on loopback HTTP.
- [ ] Validate `tcube-pi` on Raspberry Pi Zero 2 W target hardware.
- [ ] Measure Pi resource usage and admin-load impact with `just measure-pi-admin`.
- [ ] Implement Raspberry Pi GPIO input backend and validate one physical button.
- [ ] Validate MAX98357A I2S audio output and local cached-content playback on Pi Zero 2 W.
- [ ] Validate mini USB microphone capture through the Pi Zero 2 W OTG port.
- [ ] Implement LED output backend and mandatory microphone-active indication.
- [ ] Define microphone capture, retention, upload, and physical indicator privacy rules.
- [ ] Define local SQLite schema versioning and migrations.
- [ ] Add acknowledged upload and retention handling for cube event and microphone data.
- [x] Prepare GitHub Releases CI/CD for installable `tcube-pi` Pi Zero 2 W artifacts.

## Later

- [ ] Prepare full flashable SD-card image artifacts after hardware and systemd validation.
- [ ] Add content package signing, admin publication controls, retention, and rollback tooling.
- [ ] Add systemd deployment for the native Pi runtime and synchronization client.
- [ ] Revisit Docker runtime deployment after native GPIO and audio are validated.
