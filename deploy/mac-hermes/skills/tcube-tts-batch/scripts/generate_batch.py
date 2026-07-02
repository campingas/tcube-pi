#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27"]
# ///
"""Generate a batch of TTS wav files via a local Voxtral server and deliver them to a tcube-pi device.

Input is a tcube job envelope (version 1, kind tts.batch) passed as --job-json or --job-file.
For each payload item this script synthesizes one wav via POST {VOXTRAL_API_BASE}/v1/audio/speech,
uploads it to the Pi artifacts endpoint, then posts the terminal result envelope.
Re-runs are safe: artifact ids are deterministic (s1, s2, ...) and the Pi dedupes on them.

Environment:
  VOXTRAL_API_BASE      Voxtral TTS base URL, default https://127.0.0.1:11445
  VOXTRAL_API_CA_CERT   Optional CA bundle for the Voxtral endpoint
  TCUBE_PI_BASE_URL     Pi admin base URL (see tcube_client.py)
  TCUBE_ORCH_TOKEN      Pi machine bearer token
  TCUBE_PI_CA_CERT      Optional Pi CA bundle

Exit code 0 means the terminal result was delivered (even if status is failed).
A JSON summary is printed to stdout for the agent to verify.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile

import httpx

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from tcube_client import TcubeClient, TcubeClientError  # noqa: E402

DEFAULT_VOXTRAL_BASE = "https://127.0.0.1:11445"
DEFAULT_MODEL = "mistralai/Voxtral-4B-TTS-2603"
MAX_ITEMS = 50
MAX_TEXT_CHARS = 240


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--job-json", help="job envelope as a JSON string")
    group.add_argument("--job-file", help="path to a file containing the job envelope JSON")
    parser.add_argument("--model", default=os.environ.get("VOXTRAL_TTS_MODEL", DEFAULT_MODEL))
    return parser.parse_args()


def load_envelope(args: argparse.Namespace) -> dict:
    raw = args.job_json
    if args.job_file:
        with open(args.job_file, encoding="utf-8") as handle:
            raw = handle.read()
    envelope = json.loads(raw)
    if envelope.get("version") != 1:
        raise ValueError(f"unsupported envelope version: {envelope.get('version')!r}")
    if envelope.get("kind") != "tts.batch":
        raise ValueError(f"expected kind tts.batch, got {envelope.get('kind')!r}")
    if not envelope.get("job_id"):
        raise ValueError("job_id is required")
    payload = envelope.get("payload") or {}
    items = payload.get("items") or []
    if not 1 <= len(items) <= MAX_ITEMS:
        raise ValueError(f"payload.items must contain 1..{MAX_ITEMS} entries")
    for index, item in enumerate(items):
        text = (item.get("text") or "").strip()
        if not 1 <= len(text) <= MAX_TEXT_CHARS:
            raise ValueError(f"items[{index}].text must be 1..{MAX_TEXT_CHARS} characters")
    return envelope


def voxtral_client() -> httpx.Client:
    ca_path = os.environ.get("VOXTRAL_API_CA_CERT", "").strip()
    verify: bool | str = os.path.expanduser(ca_path) if ca_path else True
    return httpx.Client(timeout=120.0, verify=verify)


def check_voxtral_health(client: httpx.Client, base_url: str) -> None:
    response = client.get(f"{base_url}/health")
    response.raise_for_status()


def synthesize(client: httpx.Client, base_url: str, model: str, text: str, voice: str | None) -> str:
    body = {"model": model, "input": text, "response_format": "wav"}
    if voice:
        body["voice"] = voice
    with client.stream("POST", f"{base_url}/v1/audio/speech", json=body) as response:
        response.raise_for_status()
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as handle:
            for chunk in response.iter_bytes():
                handle.write(chunk)
            return handle.name


def main() -> int:
    args = parse_args()
    envelope = load_envelope(args)
    job_id = envelope["job_id"]
    payload = envelope["payload"]
    voice = payload.get("voice")
    language = payload.get("language", "english")
    button_id = str(payload.get("button_id", ""))
    content_type = payload.get("content_type", "language")
    voxtral_base = os.environ.get("VOXTRAL_API_BASE", DEFAULT_VOXTRAL_BASE).rstrip("/")

    pi = TcubeClient()
    tts = voxtral_client()
    artifacts: list[dict] = []
    failures: list[dict] = []
    try:
        check_voxtral_health(tts, voxtral_base)
        for index, item in enumerate(payload["items"]):
            artifact_id = f"s{index + 1}"
            text = item["text"].strip()
            wav_path = ""
            try:
                wav_path = synthesize(tts, voxtral_base, args.model, text, voice)
                fields = {
                    "content_type": content_type,
                    "button_id": button_id,
                    "language": language,
                    "text": text,
                    "title": text[:60],
                }
                uploaded = pi.post_artifact(job_id, artifact_id, fields, wav_path)
                artifacts.append(
                    {"artifact_id": artifact_id, "filename": uploaded.get("filename"), "text": text}
                )
            except (httpx.HTTPError, TcubeClientError, OSError) as error:
                failures.append({"artifact_id": artifact_id, "text": text, "error": str(error)})
            finally:
                if wav_path and os.path.exists(wav_path):
                    os.unlink(wav_path)
        status = "succeeded" if not failures else "failed"
        error_text = None
        if failures:
            error_text = "; ".join(f"{f['artifact_id']}: {f['error']}" for f in failures)[:500]
        result_envelope = {
            "version": 1,
            "job_id": job_id,
            "status": status,
            "data": {"generated": len(artifacts), "requested": len(payload["items"])},
            "artifacts": [
                {"artifact_id": a["artifact_id"], "filename": a.get("filename")} for a in artifacts
            ],
            "error": error_text,
        }
        pi.post_result(job_id, result_envelope)
        summary = {"job_id": job_id, "status": status, "artifacts": artifacts, "failures": failures}
        print(json.dumps(summary, indent=2))
        return 0
    finally:
        tts.close()
        pi.close()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # surface a machine-readable failure for the agent
        print(json.dumps({"status": "error", "error": str(error)}), file=sys.stderr)
        sys.exit(1)
