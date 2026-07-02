# Mac Mini Hermes Orchestration Stack

This directory ships the Mac Mini side of the tcube orchestration pipeline described in `docs/developer/orchestration-blueprint.md`.

Status: proposal under evaluation. This stack deviates from the current VISION.md offline-only posture; see the blueprint for the privacy boundary and the Pi-side task plan.

## What runs on the Mac Mini

- Hermes Agent (Nous Research) with the webhook adapter on port 8644 receiving HMAC-signed job envelopes from the Pi.
- Two custom skills: `tcube-tts-batch` (local Voxtral TTS batches) and `tcube-complexity-sort` (GPT-5.5 analysis via the openai-codex provider).
- A local Voxtral TTS server (OpenAI-compatible) on `https://127.0.0.1:11445`, matching the contract the Pi already uses in `src/server/speech.rs`.

## Install

1. Install Hermes Agent per the official docs, then authenticate the Codex subscription: `hermes auth add codex-oauth`.
2. Merge `config/hermes-config-fragment.yaml` into `~/.hermes/config.yaml`.
3. Copy the needed lines from `config/env.example` into `~/.hermes/.env` and generate real secrets with `openssl rand -hex 32`.
4. Install the skills by copying or symlinking `skills/tcube-tts-batch` and `skills/tcube-complexity-sort` into your Hermes skills directory (or `hermes skills install <path>`).
5. Export the Pi Caddy internal root CA and store it as `~/.hermes/tcube-pi-ca.pem`. On the Pi: `sudo caddy trust --address unix//run/caddy-admin.sock` documents the CA location; the root certificate lives at `/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt`, copy it over with `scp`.
6. Start the Voxtral server: `voxtral/serve-voxtral-tts`, and front it with Caddy tls internal (`voxtral/serve-voxtral-tts caddyfile` prints the snippet).
7. Restart Hermes so the webhook route and skills load.

## Smoke test

Send a fake job through the webhook route (replace the secret):

```sh
body='{"version":1,"job_id":"smoke-1","kind":"tts.batch","created_at":"2026-07-02T00:00:00Z","attempt":1,"reply_to":{"base_url":"https://tcube.local","artifacts_path":"/api/pi/v1/orch/jobs/smoke-1/artifacts","result_path":"/api/pi/v1/orch/jobs/smoke-1/result"},"payload":{"button_id":1,"language":"english","voice":null,"items":[{"text":"Hello little explorer"}]}}'
sig=$(printf '%s' "$body" | openssl dgst -sha256 -hmac "$TCUBE_WEBHOOK_SECRET" -hex | awk '{print $2}')
curl -s -X POST http://127.0.0.1:8644/webhooks/tcube-jobs \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Signature: $sig" \
  -d "$body"
```

A 2xx response means the job was accepted; completion is delivered asynchronously to the Pi ingest API. `hermes webhook test tcube-jobs --payload "$body"` exercises the route without HTTP.

## Notes

- The Pi is the state authority and idempotency owner (`job_id`, `artifact_id`); re-delivery is always safe.
- Skills must never call content activation endpoints; generated audio stays draft until a parent activates it.
- Audio never leaves the LAN; only sentence text reaches GPT-5.5 through the Codex path.
