"""Shared HTTP client for delivering orchestration results to a tcube-pi device.

Environment:
  TCUBE_PI_BASE_URL   Base URL of the Pi admin API through Caddy, for example https://tcube.local
  TCUBE_ORCH_TOKEN    Machine bearer token expected by the Pi ingest endpoints
  TCUBE_PI_CA_CERT    Optional path to the Pi Caddy internal root CA (PEM)

All POSTs are idempotent on the Pi side (job_id plus artifact_id), so retries are safe.
"""

from __future__ import annotations

import os
import time

import httpx

DEFAULT_TIMEOUT_SECONDS = 30.0
RETRIES = 3
BACKOFF_BASE_SECONDS = 2.0


class TcubeClientError(RuntimeError):
    """Raised when the Pi ingest API cannot be reached or rejects a request."""


def _require_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise TcubeClientError(f"missing required environment variable {name}")
    return value


class TcubeClient:
    def __init__(
        self,
        base_url: str | None = None,
        token: str | None = None,
        ca_cert: str | None = None,
    ) -> None:
        self.base_url = (base_url or _require_env("TCUBE_PI_BASE_URL")).rstrip("/")
        token_value = token or _require_env("TCUBE_ORCH_TOKEN")
        ca_path = ca_cert or os.environ.get("TCUBE_PI_CA_CERT", "").strip()
        verify: bool | str = True
        if ca_path:
            expanded = os.path.expanduser(ca_path)
            if not os.path.isfile(expanded):
                raise TcubeClientError(f"TCUBE_PI_CA_CERT points to a missing file: {expanded}")
            verify = expanded
        self._client = httpx.Client(
            timeout=DEFAULT_TIMEOUT_SECONDS,
            verify=verify,
            headers={"Authorization": f"Bearer {token_value}"},
        )

    def close(self) -> None:
        self._client.close()

    def _post_with_retry(self, url: str, **kwargs) -> httpx.Response:
        last_error = "unknown error"
        for attempt in range(1, RETRIES + 1):
            try:
                response = self._client.post(url, **kwargs)
                if response.status_code < 500:
                    return response
                last_error = f"http {response.status_code}: {response.text[:200]}"
            except httpx.HTTPError as error:
                last_error = str(error)
            if attempt < RETRIES:
                time.sleep(BACKOFF_BASE_SECONDS * (2 ** (attempt - 1)))
        raise TcubeClientError(f"POST {url} failed after {RETRIES} attempts: {last_error}")

    def post_artifact(
        self,
        job_id: str,
        artifact_id: str,
        fields: dict[str, str],
        wav_path: str,
    ) -> dict:
        """Upload one draft audio artifact. Duplicate artifact_id returns the existing item (200)."""
        url = f"{self.base_url}/api/pi/v1/orch/jobs/{job_id}/artifacts"
        data = {"artifact_id": artifact_id, **fields}
        with open(wav_path, "rb") as handle:
            files = {"audio_file": (os.path.basename(wav_path), handle, "audio/wav")}
            response = self._post_with_retry(url, data=data, files=files)
        if response.status_code not in (200, 201):
            raise TcubeClientError(
                f"artifact {artifact_id} rejected: http {response.status_code}: {response.text[:200]}"
            )
        return response.json()

    def post_result(self, job_id: str, envelope: dict) -> dict:
        """Deliver the terminal job result. Duplicate delivery of the same status returns 200."""
        url = f"{self.base_url}/api/pi/v1/orch/jobs/{job_id}/result"
        response = self._post_with_retry(url, json=envelope)
        if response.status_code not in (200, 201):
            raise TcubeClientError(
                f"result for job {job_id} rejected: http {response.status_code}: {response.text[:200]}"
            )
        return response.json()
