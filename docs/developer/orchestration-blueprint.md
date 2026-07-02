# Orchestration Blueprint: Pi + Mac Mini (Hermes Agent)

Status: proposal under evaluation, 2026-07-02. This blueprint deviates from the VISION.md offline-only posture (principles 5 and 6 and the "no cloud dependency" contract) by design; it exists to evaluate a Mac Mini orchestration path before any product decision. Nothing here is enabled by default, and the Pi-side Rust work is not implemented yet (see the task plan below and `docs/tasks.md`).

## Goal

Let the Pi delegate heavy content work to a Mac Mini running Nous Research's Hermes Agent, which routes text-to-speech to a local Voxtral server and analytical work to GPT-5.5 through the Hermes `openai-codex` provider (ChatGPT/Codex subscription, device-code OAuth, no per-token billing).

Non-negotiables preserved from the product contract:

- The child path stays fully offline and under 100 ms; orchestration lives only in the `tcube-pi-admin` process, never the runtime binary.
- The Pi stays state-authoritative; the Mac is optional compute. Everything Hermes returns lands as a draft behind the existing parent activation gate.
- The Pi keeps working when the Mac or internet is down: jobs wait in a persistent SQLite outbox and retry with backoff.

## Privacy boundary

When a parent enables orchestration (`TCUBE_ORCH_ENABLED=1` plus explicit secrets), sentence text leaves the Pi for the Mac Mini on the home LAN. For analysis jobs, sentence text (never audio) leaves the home to OpenAI through the Codex path. Audio never leaves the LAN. Orchestration is off by default and requires deliberate configuration on both machines.

## Topology

```
+---------------------------+          LAN            +----------------------------------+
| Pi Zero 2W                |                         | Mac Mini                         |
|                           | 1. POST (HMAC-signed)   |                                  |
| tcube-pi (runtime)        |  /webhooks/tcube-jobs   | Hermes Agent gateway :8644       |
|   offline, <100ms, GPIO   | ----------------------> |   route -> prompt + skills       |
|                           |                         |   |- tcube-tts-batch skill       |
| tcube-pi-admin (axum)     | 2. artifacts + result   |   |    -> Voxtral TTS :11445     |
|   |- job outbox (SQLite)  | <---------------------- |   |       (OpenAI-compat, local) |
|   |- ingest API (bearer)  |  Bearer TCUBE_ORCH_TOKEN|   |- tcube-complexity-sort skill |
|   |- draft/active gate    |  via Caddy HTTPS        |        -> openai-codex gpt-5.5   |
+---------------------------+                         +----------------------------------+
```

Jobs are Pi-initiated: the Pi owns the queue, the retries, and idempotency (`job_id`, `artifact_id`). Results are Mac-pushed to the Pi ingest API through the existing Caddy HTTPS front. The Mac side ships in `deploy/mac-hermes/`.

## Wire contract (version 1)

### Pi to Mac: job envelope

`POST http://<mac-host>:8644/webhooks/tcube-jobs` with header `X-Webhook-Signature` carrying the hex HMAC-SHA256 of the body under the shared secret.

```json
{
  "version": 1,
  "job_id": "0197c2f4-7f3e-7bb1-a2d0-5c9e8f3a1b42",
  "kind": "tts.batch",
  "created_at": "2026-07-02T14:31:07Z",
  "attempt": 1,
  "reply_to": {
    "base_url": "https://tcube.local",
    "artifacts_path": "/api/pi/v1/orch/jobs/{job_id}/artifacts",
    "result_path": "/api/pi/v1/orch/jobs/{job_id}/result"
  },
  "payload": {}
}
```

`tts.batch` payload: `{"button_id": 1, "language": "english", "voice": "warm-narrator", "items": [{"text": "..."}]}` with 1 to 50 items and 1 to 240 characters per text, matching the existing generated-speech limits.

`analysis.complexity_sort` payload: `{"language": "english", "items": [{"id": "<content_item_id or filename>", "text": "..."}]}`. The `text` field is authoritative; the generated filename convention `generated-{model}-{language}-{slug}-{timestamp}.wav` (`src/server/media.rs`) is the fallback text source.

### Mac to Pi: artifact delivery

`POST /api/pi/v1/orch/jobs/{job_id}/artifacts` with `Authorization: Bearer $TCUBE_ORCH_TOKEN`, multipart fields `artifact_id`, `content_type`, `button_id`, `language`, `text`, `title`, `audio_file`. The Pi lands artifacts through the existing draft pipeline (`source='generated'`, file under `data/audio/draft/...`), so WAV validation and the activation gate apply unchanged. Idempotent on `artifact_id`.

### Mac to Pi: terminal result

`POST /api/pi/v1/orch/jobs/{job_id}/result` with JSON `{"version": 1, "job_id": "...", "status": "succeeded|failed", "data": {...}, "artifacts": [{"artifact_id": "s1", "filename": "..."}], "error": null}`. For complexity sorting, `data.sorted` holds `[{"rank", "id", "file", "complexity", "score"}]`.

### Semantics

A webhook 2xx means accepted, not done. Delivery is at-least-once; `job_id` and `artifact_id` are the idempotency keys. Duplicate terminal results return 200; conflicting terminal status returns 409. Mac-side skills retry idempotent POSTs three times with backoff, then post a failed result rather than leaving a job hanging.

## Mac Mini stack

Everything ships in `deploy/mac-hermes/`: a Hermes `config.yaml` fragment (webhook route plus `openai-codex` model routing), `env.example`, the `tcube-tts-batch` and `tcube-complexity-sort` skills with Python helper scripts, and a `serve-voxtral-tts` launcher. See `deploy/mac-hermes/README.md` for install and smoke-test steps.

Routing rule: TTS never rides an LLM route; the skill script calls Voxtral `/v1/audio/speech` directly (the same OpenAI-compatible contract `src/server/speech.rs` already uses). Only the analysis skill uses a model, and it rides the Hermes session main model on `openai-codex/gpt-5.5`, so the Codex subscription needs no custom dispatch code.

File-transfer strategy: direct streamed HTTPS multipart per artifact. Short wav files are roughly 100 to 300 KB; per-artifact atomicity with idempotent retry beats bulk archives or rsync at this size, reuses the single Caddy/bearer auth model, and avoids handing SSH keys to an agent. Per-item POSTs stay under the Pi body limit by construction.

## Pi-side Rust plan (not yet implemented)

Design anchors verified against the code: `content_jobs` CHECK constraints (`src/db/admin/schema.rs:110`) do not fit the orchestration lifecycle, so a new table is required; `save_multipart_media` (`src/server/routes/content.rs:292`) contains the reusable draft-save path; `media_input_from_axum_multipart` (`src/server/media.rs:39`) parses the multipart fields; migrations currently record versions 1 to 5 (`schema.rs:290`).

- New tables `orchestration_jobs` (uuid id, kind, status in queued/dispatched/running/succeeded/failed/dead/cancelled, JSON payload and result_data, attempts, max_attempts, next_attempt_at, timestamps) and `orchestration_job_artifacts` (artifact dedupe plus `content_items` link), as migration version 6 with a dispatch index. New `src/db/admin/jobs.rs` storage module.
- `OrchestratorConfig` as `Option` on `AdminConfig`, entirely off by default. Env-driven: `TCUBE_ORCH_ENABLED=1`, `HERMES_WEBHOOK_URL`, `HERMES_WEBHOOK_SECRET`, `TCUBE_ORCH_TOKEN` (32 bytes minimum), `TCUBE_ORCH_REPLY_BASE`, interval/attempt/deadline knobs, optional CA cert. Documented commented block in `deploy/pi-admin-caddy/tcube-pi-admin.env`. Cargo additions: `hmac`, tokio `time` feature.
- New module `src/server/orchestrator/{mod,envelope,signing,dispatch}.rs` plus `src/server/routes/jobs.rs`. Wire types mirror this document exactly. Signing is one function (`sha256=` prefix plus hex HMAC of the body) so the exact Hermes adapter scheme stays a one-line adjustment.
- Routes. Human (session cookie): `POST/GET /api/pi/v1/jobs`, `GET /api/pi/v1/jobs/{id}`, `GET /api/pi/v1/orch/status`. Machine (new `OrchBearer` extractor mirroring `SessionCookie`, constant-time compare): the two ingest endpoints above. Disabled feature returns 503; bad token returns 401.
- Ingest reuses the draft pipeline via a pure-extraction refactor: split `save_multipart_media` into auth plus a shared `save_media_draft(conn, config, input, source)`; ingest calls it with `source="generated"`, preserving WAV validation and the activation gate. `MediaInput` gains an optional `artifact_id`.
- Dispatcher: a tokio background task spawned in `server::run` only when orchestration is configured. Each tick runs synchronously inside `spawn_blocking` (per-tick rusqlite connection, blocking reqwest, 5 s connect and 30 s request timeouts). Success moves jobs to running; failure reschedules with exponential backoff (30 s doubling, 15 min cap, jitter); exhausted attempts mark jobs dead; stuck running jobs are reaped after a deadline. The runtime binary is untouched.

Ordered tasks (each roughly one PR, strict order 1, 2, 3, then 4 and 5 in parallel, then 6, 7):

1. Schema migration plus `src/db/admin/jobs.rs` storage layer with unit tests.
2. Config plumbing (`src/config.rs`, `src/bin/tcube_pi_admin.rs`, env file, Cargo deps; `orchestration: None` added to every test `AdminConfig` constructor).
3. Contract and signing module with serde and HMAC-vector unit tests.
4. Draft-save refactor in `content.rs` and `media.rs` with zero behavior change.
5. Admin job endpoints plus `ApiError` variants for not-found, conflict, and service-unavailable.
6. Machine ingest endpoints with integration tests: token cases, idempotency, and an end-to-end test proving ingested drafts still require parent activation.
7. Dispatcher with a mock-Hermes integration test (axum mock on `127.0.0.1:0` capturing `X-Webhook-Signature`, driving `dispatch_tick` directly) and an offline-tolerance test.

Open item: confirm whether the Hermes webhook adapter validates the HMAC over the raw body only or over a timestamped variant; the signing helper isolates this to one line.

## Failure model

- Mac down: jobs stay queued on the Pi; the dispatcher backs off and resumes automatically. The child path is unaffected.
- Voxtral down: the TTS skill posts a failed result immediately; the Pi marks the job failed and the parent can retry.
- Frontier path down: the analysis skill posts a failed result; no partial state lands on the Pi.
- Pi unreachable during delivery: skills retry three times, then the job is reaped on the Pi after the running deadline and re-dispatched.
- Duplicate or replayed deliveries: idempotency keys make them safe; conflicting terminal states surface as 409 and stop the skill.

## Phase 2 (future, not built)

An MCP server wrapping the Pi's read-only API (inventory, events, job status) so interactive Hermes sessions can browse device state as tools. Deferred until the webhook pipeline proves out.
