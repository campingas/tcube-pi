---
name: tcube-tts-batch
description: Generate TTS wav batches via local Voxtral for tcube-pi
version: 1.0.0
license: Apache-2.0
platforms: [macos, linux]
metadata:
  hermes:
    tags: [Audio, TTS, TCube, Orchestration]
    related_skills: [tcube-complexity-sort]
required_environment_variables:
  - name: TCUBE_PI_BASE_URL
    prompt: "Base URL of the tcube-pi admin API"
    help: "The Caddy HTTPS front on the Pi, for example https://tcube.local"
    required_for: "delivering artifacts and results to the Pi"
  - name: TCUBE_ORCH_TOKEN
    prompt: "tcube-pi orchestration bearer token"
    help: "Must match TCUBE_ORCH_TOKEN in the Pi tcube-pi-admin.env (32 bytes minimum)"
    required_for: "authenticating against the Pi ingest endpoints"
  - name: VOXTRAL_API_BASE
    prompt: "Voxtral TTS base URL"
    help: "Local OpenAI-compatible TTS server, for example https://127.0.0.1:11445"
    required_for: "speech synthesis"
required_credential_files:
  - path: tcube-pi-ca.pem
    description: "tcube-pi Caddy internal root CA used to verify HTTPS to the Pi (set TCUBE_PI_CA_CERT to its path)"
---

# tcube TTS Batch

## When to Use

Use this skill when a tcube-jobs webhook delivers a job envelope with kind `tts.batch`. The device requests a batch of short sentences rendered as wav files with a specific voice and returned to the Pi as draft content.

## Quick Reference

| Action | Command |
| --- | --- |
| Voxtral health | `curl -sk $VOXTRAL_API_BASE/health` |
| Run the batch | `uv run ${HERMES_SKILL_DIR}/scripts/generate_batch.py --job-json '<envelope>'` |
| List voices | `curl -sk $VOXTRAL_API_BASE/v1/audio/voices` |

## Procedure

1. Confirm the webhook payload is a complete version 1 envelope with kind `tts.batch`, a `job_id`, and 1 to 50 items each with non-empty `text` (240 characters maximum).
2. Check Voxtral health first: `curl -sk $VOXTRAL_API_BASE/health`. If unhealthy, skip synthesis and go to step 5.
3. Write the envelope JSON to a temp file, then run `uv run ${HERMES_SKILL_DIR}/scripts/generate_batch.py --job-file <path>`. The script synthesizes one wav per item, uploads each to the Pi artifacts endpoint with bearer auth, and posts the terminal result envelope.
4. Verify the printed JSON summary: `status` is `succeeded` and the `artifacts` array has one entry per requested item. If some items failed, the script already reported `status: failed` with details; do not retry more than once.
5. If synthesis is impossible (Voxtral down, invalid payload), post a terminal failure yourself so the Pi does not wait: `POST {TCUBE_PI_BASE_URL}/api/pi/v1/orch/jobs/{job_id}/result` with `Authorization: Bearer $TCUBE_ORCH_TOKEN` and body `{"version": 1, "job_id": "<id>", "status": "failed", "data": null, "artifacts": [], "error": "<reason>"}`.

## Pitfalls

- Never activate content on the Pi. Artifacts land as drafts behind the parent activation gate by design; activation endpoints are session-only and off-limits to this skill.
- The webhook 200 acknowledgment is not job completion; the Pi only considers the job done when the result envelope arrives.
- Re-running the script for the same job is safe: artifact ids are deterministic (`s1`, `s2`, ...) and the Pi dedupes on them.
- Audio must stay on the LAN. Do not attach wav files or sentence text to any cloud model call from this skill.
- One wav per sentence; do not concatenate items into a single file.

## Verification

The script summary lists every artifact with the Pi-assigned filename. Cross-check `artifacts` length against `payload.items` length and confirm the final result POST returned 200.
