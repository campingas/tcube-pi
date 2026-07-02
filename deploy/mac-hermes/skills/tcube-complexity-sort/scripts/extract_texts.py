#!/usr/bin/env python3
"""Normalize a tcube analysis.complexity_sort job payload into {id, text, file} triples.

Input is a tcube job envelope (version 1, kind analysis.complexity_sort) via --job-json or --job-file.
The payload text field is authoritative. When an item has a file but no text, the text is
recovered from the tcube generated filename convention:
  generated-{model}-{language}-{text-slug}-{timestamp}.wav
The timestamp is 17 digits (%Y%m%d%H%M%S%3f). The slug is de-hyphenated into words.

Output (stdout): JSON {"job_id", "criteria", "order", "items": [{"id", "text", "file", "text_source"}]}.
The agent ranks these items by wording complexity and posts the result to the Pi.
Stdlib only, no third-party dependencies.
"""

from __future__ import annotations

import argparse
import json
import re
import sys

GENERATED_PATTERN = re.compile(r"^generated-(?P<body>.+)-(?P<timestamp>\d{17})\.(wav|mp3)$")
KNOWN_LANGUAGES = (
    "english",
    "spanish",
    "french",
    "german",
    "italian",
    "portuguese",
    "dutch",
    "hindi",
    "vietnamese",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--job-json", help="job envelope as a JSON string")
    group.add_argument("--job-file", help="path to a file containing the job envelope JSON")
    return parser.parse_args()


def text_from_filename(filename: str) -> str | None:
    match = GENERATED_PATTERN.match(filename)
    if not match:
        return None
    body = match.group("body")
    for language in KNOWN_LANGUAGES:
        marker = f"-{language}-"
        position = body.rfind(marker)
        if position != -1:
            slug = body[position + len(marker):]
            return slug.replace("-", " ").strip() or None
    return None


def main() -> int:
    args = parse_args()
    raw = args.job_json
    if args.job_file:
        with open(args.job_file, encoding="utf-8") as handle:
            raw = handle.read()
    envelope = json.loads(raw)
    if envelope.get("version") != 1:
        raise ValueError(f"unsupported envelope version: {envelope.get('version')!r}")
    if envelope.get("kind") != "analysis.complexity_sort":
        raise ValueError(f"expected kind analysis.complexity_sort, got {envelope.get('kind')!r}")
    payload = envelope.get("payload") or {}
    items_in = payload.get("items") or []
    if not items_in:
        raise ValueError("payload.items must not be empty")

    items_out = []
    for index, item in enumerate(items_in):
        text = (item.get("text") or "").strip()
        file_name = (item.get("file") or "").strip()
        source = "payload"
        if not text and file_name:
            recovered = text_from_filename(file_name)
            if recovered:
                text = recovered
                source = "filename"
        if not text:
            raise ValueError(f"items[{index}] has no text and no parsable filename: {file_name!r}")
        items_out.append(
            {
                "id": item.get("id") or file_name or f"item-{index + 1}",
                "text": text,
                "file": file_name or None,
                "text_source": source,
            }
        )

    print(
        json.dumps(
            {
                "job_id": envelope.get("job_id"),
                "criteria": payload.get("criteria", "wording_complexity"),
                "order": payload.get("order", "ascending"),
                "items": items_out,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:
        print(json.dumps({"status": "error", "error": str(error)}), file=sys.stderr)
        sys.exit(1)
