# Pi Release Bundle

This directory contains release-bundle helper scripts for Raspberry Pi OS Lite 64-bit on Pi Zero 2 W.

Use `just prepare-release vX.Y.Z` before creating a release tag. The recipe calls `deploy/pi-release/prepare-release`, updates `Cargo.toml`, `Cargo.lock`, and `admin-ui/package.json`, and validates the updated lockfiles.

`install-on-pi` is copied into GitHub Release tarballs as root-level `install.sh`. Run that packaged installer on the Pi with `sudo ./install.sh` after verifying `SHA256SUMS`.

`install-latest` is the curl-pipe bootstrapper documented in `docs/hardware/pi-os-lite-install.md`. It resolves the latest release, downloads the archive and `SHA256SUMS`, verifies them, extracts the bundle, and runs the packaged `install.sh`. When the resolved version matches `/opt/tcube/VERSION` it exits early without downloading; set `TCUBE_PI_FORCE=1` to reinstall the same version.

Update behavior on an already-installed Pi:

- The packaged installer records the installed version in `/opt/tcube/VERSION`.
- `tcube-pi-admin.service` and `tcube-mdns-alias.service` are restarted only when their binary, unit file, or env file changed.
- An existing `/etc/tcube/tcube-pi-admin.env` is kept as-is; the new release defaults are written next to it as `tcube-pi-admin.env.dist`.
- Data under `/var/lib/tcube` (database and media) is never touched.

Keep release-bundle scripts here. Keep long-running service files and Caddy configuration in `deploy/pi-admin-caddy/`.
