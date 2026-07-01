# Pi Release Bundle

This directory contains release-bundle helper scripts for Raspberry Pi OS Lite 64-bit on Pi Zero 2 W.

Use `just prepare-release vX.Y.Z` before creating a release tag. The recipe calls `deploy/pi-release/prepare-release`, updates `Cargo.toml`, `Cargo.lock`, and `admin-ui/package.json`, and validates the updated lockfiles.

`install-on-pi` is copied into GitHub Release tarballs as root-level `install.sh`. Run that packaged installer on the Pi with `sudo ./install.sh` after verifying `SHA256SUMS`.

`install-latest` is the curl-pipe bootstrapper documented in `docs/hardware/pi-os-lite-install.md`. It resolves the latest release, downloads the archive and `SHA256SUMS`, verifies them, extracts the bundle, and runs the packaged `install.sh`.

Keep release-bundle scripts here. Keep long-running service files and Caddy configuration in `deploy/pi-admin-caddy/`.
