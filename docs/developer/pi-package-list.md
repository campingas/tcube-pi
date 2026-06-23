# Raspberry Pi Package List

This file tracks packages and host tools expected on the Raspberry Pi. Keep it aligned with the actual runtime and deployment path as hardware support is added.

## Target OS

Use Raspberry Pi OS Lite 64-bit on Raspberry Pi Zero 2 W.

As of 2026-06-11, Raspberry Pi lists the current 64-bit Lite image as Debian 13 Trixie with no desktop environment.

## Required Base Packages

Install these before building or running T-Cube on the Pi:

```sh
sudo apt update
sudo apt install -y ca-certificates caddy curl git just sqlite3
```

Purpose:

* `ca-certificates`: TLS certificate roots for package and source downloads.
* `caddy`: HTTPS reverse proxy for the Pi-hosted admin service.
* `curl`: installer and download helper.
* `git`: source checkout.
* `just`: repo command runner.
* `sqlite3`: local database inspection and maintenance.

## Native Build Packages

Install these if building the Rust runtime directly on the Pi:

```sh
sudo apt install -y build-essential pkg-config
```

Install Rust using the official Rust toolchain installer unless the project later pins another method:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Purpose:

* `build-essential`: native compiler/linker toolchain.
* `pkg-config`: native library discovery for Rust crates with system dependencies.
* Rust toolchain: builds `tcube-pi`.

## Admin UI/API Packages

Do not install Node.js, pnpm, Vite, or Hono on the Pi Zero 2 W. The Pi admin workflow is Rust-native.

Python model environments, MLX, Voxtral, and Vietnamese VITS are Mac mini dependencies, not cube dependencies.

The Pi-hosted Rust admin service is the `tcube-pi` binary `tcube-pi-admin`. It owns the local admin API surface and retains legacy static-file compatibility; it does not require Node.js, pnpm, Vite, Hono, Python, or model workers on the Pi.

For HTTPS, run `tcube-pi-admin` on loopback HTTP behind Caddy. The deployment sketch under `deploy/pi-admin-caddy/` uses `127.0.0.1:8080` for Rust and Caddy `tls internal` for `tcube.local` and `10.55.0.1`. Admin phones and laptops must trust Caddy's internal root CA before browser recording and secure cookies are realistic.

## Audio Packages

Audio output is implemented in the Rust runtime with `rodio`, which uses the default OS audio output device.

Install ALSA diagnostics on Raspberry Pi OS before validating speaker or amplifier behavior:

```sh
sudo apt install -y alsa-utils
```

Purpose:

* `alsa-utils`: sound device inspection, mixer control, and command-line playback diagnostics.

The selected prototype output path is a MAX98357A I2S amplifier and mono 3 W speaker. The selected input path is the mini USB microphone connected through the Pi Zero 2 W USB OTG port. Validate simultaneous playback and capture before implementing background recording.

For the intermediate one-button smoke payload under `deploy/pi-zero-button-smoke`, also install:

```sh
sudo apt install -y gpiod mpg123
```

Purpose:

* `gpiod`: `gpiomon` command-line GPIO edge detection for the temporary physical-button smoke test.
* `mpg123`: command-line MP3 playback for stock music content while the final Rust Pi GPIO backend is pending.

## GPIO Packages

GPIO support is not implemented yet. Final packages should be selected when the Rust GPIO crate and access model are chosen.

Likely host requirements:

* user membership or service permissions for GPIO access
* access to GPIO character devices such as `/dev/gpiochip*`

## Optional Docker Dev Tooling

Docker is optional on the Pi for now. Use it as a reproducible development shell, not as the first hardware runtime path.

Follow Docker's Debian install documentation for Docker Engine and the Compose plugin. Raspberry Pi OS is Debian-based, and current 64-bit Lite images use arm64 Debian Trixie.

Do not make the child-facing runtime depend on Docker until native GPIO and audio behavior have been validated.
