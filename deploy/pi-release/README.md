# Pi Release Bundle

This directory contains release-bundle helper scripts for Raspberry Pi OS Lite 64-bit on Pi Zero 2 W.

Use `just prepare-release vX.Y.Z` before creating a release tag. The recipe calls `deploy/pi-release/prepare-release`, updates `Cargo.toml`, `Cargo.lock`, and `admin-ui/package.json`, and validates the updated lockfiles.

`install-on-pi` is copied into GitHub Release tarballs as root-level `install.sh`. Run that packaged installer on the Pi with `sudo ./install.sh` after verifying `SHA256SUMS`.

`install-latest` is the curl-pipe bootstrapper documented in `docs/hardware/pi-os-lite-install.md`. It resolves the latest release, downloads the archive and `SHA256SUMS`, verifies them, extracts the bundle, and runs the packaged `install.sh`. When the resolved version matches `/opt/tcube/VERSION` it exits early without downloading; set `TCUBE_PI_FORCE=1` to reinstall the same version.

Update behavior on an already-installed Pi:

- The packaged installer records the installed version in `/opt/tcube/VERSION`.
- Before writing application files, the installer checks the active `wlan0` profile. A profile backed by `/etc/NetworkManager/system-connections/` is left untouched; a temporary profile is cloned through NetworkManager to the persistent `tcube-wifi` profile without activating it.
- The installer refuses to replace an unrelated existing `tcube-wifi` profile and validates that a new clone is root-owned, mode `600`, autoconnect-enabled with priority `100`, and backed by `/etc/NetworkManager/system-connections/`.
- A journald drop-in enables persistent logs with a `64M` cap; the fresh-install guide defines the reboot and reconnect acceptance gate.
- `tcube-pi-admin.service` and `tcube-mdns-alias.service` are restarted only when their binary, unit file, or env file changed.
- An existing `/etc/tcube/tcube-pi-admin.env` is kept as-is; the new release defaults are written next to it as `tcube-pi-admin.env.dist`.
- Data under `/var/lib/tcube` (database and media) is never touched.

Keep release-bundle scripts here. Keep long-running service files and Caddy configuration in `deploy/pi-admin-caddy/`.

Run `just test-pi-installer` for fixture coverage of the Wi-Fi safeguard. The test does not activate, reconnect, or otherwise modify a live network connection.
