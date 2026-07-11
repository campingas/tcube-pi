---
name: tcube-complexity-sort
description: Sort tcube sentences by wording complexity via GPT-5.5
version: 1.0.0
license: Apache-2.0
platforms: [macos, linux]
metadata:
  hermes:
    tags: [Analysis, NLP, TCube, Orchestration]
    related_skills: [tcube-tts-batch]
required_environment_variables:
  - name: TCUBE_PI_BASE_URL
    prompt: "Base URL of the tcube-pi admin API"
    help: "The Caddy HTTPS front on the Pi, for example https://tcube.local"
    required_for: "delivering the sorted result to the Pi"
  - name: TCUBE_ORCH_TOKEN
    prompt: "tcube-pi orchestration bearer token"
    help: "Must match TCUBE_ORCH_TOKEN in the Pi tcube-pi-admin.env (32 bytes minimum)"
    required_for: "authenticating against the Pi ingest endpoints"
required_credential_files:
  - path: tcube-pi-ca.pem
    description: "tcube-pi Caddy internal root CA used to verify HTTPS to the Pi (set TCUBE_PI_CA_CERT to its path)"
---

# tcube Complexity Sort

## When to Use

Use this skill when a tcube-jobs webhook delivers a job envelope with kind `analysis.complexity_sort`. The device asks for its generated sentences to be ranked by wording complexity. This analysis runs on the session main model, which is routed through the openai-codex provider (GPT-5.5 on the Codex subscription); only sentence text is analyzed, never audio.

## Quick Reference

| Action | Command |
| --- | --- |
| Normalize payload | `python3 ${HERMES_SKILL_DIR}/scripts/extract_texts.py --job-json '<envelope>'` |
| Deliver result | `POST {TCUBE_PI_BASE_URL}/api/pi/v1/orch/jobs/{job_id}/result` with bearer token |

## Procedure

1. Confirm the webhook payload is a complete version 1 envelope with kind `analysis.complexity_sort` and a non-empty `payload.items` list.
2. Run `python3 ${HERMES_SKILL_DIR}/scripts/extract_texts.py --job-file <path>` on the envelope. It outputs normalized `{id, text, file, text_source}` items, recovering text from the tcube filename convention (`generated-{model}-{language}-{slug}-{timestamp}.wav`) when the payload text is missing.
3. Rank the normalized items by wording complexity yourself. Consider vocabulary rarity, syntactic depth, and sentence length. Honor `order` (`ascending` means simplest first). Produce strict JSON only, shaped exactly as: `{"sorted": [{"rank": 1, "id": "<id>", "file": "<file or null>", "complexity": "low|medium|high", "score": <0-100 integer>}]}`.
4. Validate your own output before sending: every input id appears exactly once, ranks are contiguous starting at 1, and scores are monotonic with rank order.
5. Deliver the terminal result with curl: `POST {TCUBE_PI_BASE_URL}/api/pi/v1/orch/jobs/{job_id}/result`, headers `Authorization: Bearer $TCUBE_ORCH_TOKEN` and `Content-Type: application/json` (add `--cacert $TCUBE_PI_CA_CERT` when set), body `{"version": 1, "job_id": "<id>", "status": "succeeded", "data": {"sorted": [...], "model": "gpt-5.5", "route": "openai-codex"}, "artifacts": [], "error": null}`.
6. If normalization fails or items cannot be ranked, post `status: failed` with the reason in `error` instead of leaving the job hanging.

## Pitfalls

- The `text` field in the payload is authoritative; the filename slug is only a fallback and loses punctuation and casing.
- Do not upload audio anywhere and do not fetch wav bytes; this job is text-only by contract.
- Duplicate result delivery with the same status returns 200 (safe); a conflicting terminal status returns 409, which means the job already completed differently. Stop and report instead of retrying.
- Keep the result JSON strict: no commentary, no markdown fences, no extra keys inside `sorted` entries.

## Verification

The Pi responds 200 to the result POST and `GET {TCUBE_PI_BASE_URL}/api/pi/v1/jobs/{job_id}` (session-authenticated, for humans) shows status `succeeded` with the stored ordering.
