#!/usr/bin/env python3
"""
Minimal Nod issuer sender.

Examples:
  NOD_BASE_URL=http://127.0.0.1:8767 NOD_ISSUER_TOKEN=... ./issuer.py "Approve deploy"
  echo "Longer **markdown** body" | ./issuer.py "Build finished" --informational
"""

import argparse
import json
import os
import sys
import urllib.error
import urllib.request


def first_env(*names):
    for name in names:
        value = os.environ.get(name)
        if value:
            return value
    return ""


def requests_url(base_url):
    base_url = base_url.rstrip("/")
    if base_url.endswith("/api/v1/requests"):
        return base_url
    return f"{base_url}/api/v1/requests"


def main():
    parser = argparse.ArgumentParser(description="Send one request to Nod.")
    parser.add_argument("title", help="request title")
    parser.add_argument("body", nargs="?", help="request body; stdin is used when omitted")
    parser.add_argument("--url", default=first_env("NOD_URL", "NOD_BASE_URL"))
    parser.add_argument("--token", default=first_env("NOD_ISSUER_TOKEN", "NOD_TOKEN"))
    parser.add_argument("--source", default=first_env("NOD_SOURCE_ID") or "default")
    parser.add_argument("--summary", default="")
    parser.add_argument("--redact-apns", action="store_true", help="send generic APNs alert text")
    parser.add_argument("--apns-title", default="", help="custom APNs alert title")
    parser.add_argument("--apns-body", default="", help="custom APNs alert body")
    parser.add_argument("--dedupe-key", default="")
    parser.add_argument("--informational", action="store_true", help="send without approval options")
    args = parser.parse_args()

    if not args.url:
        parser.error("missing --url or NOD_URL/NOD_BASE_URL")
    if not args.token:
        parser.error("missing --token or NOD_ISSUER_TOKEN/NOD_TOKEN")

    body = args.body
    if body is None and not sys.stdin.isatty():
        body = sys.stdin.read().strip()

    payload = {"source_id": args.source, "title": args.title}
    if args.summary:
        payload["summary"] = args.summary
    if body:
        payload["body_markdown"] = body
    notification = {}
    if args.redact_apns:
        notification["redact"] = True
    if args.apns_title:
        notification["title"] = args.apns_title
    if args.apns_body:
        notification["body"] = args.apns_body
    if notification:
        payload["notification"] = notification
    if args.dedupe_key:
        payload["dedupe_key"] = args.dedupe_key
    if not args.informational:
        payload["options"] = [
            {"id": "approve", "label": "Approve", "kind": "approve"},
            {"id": "reject", "label": "Reject", "kind": "reject"},
        ]

    request = urllib.request.Request(
        requests_url(args.url),
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Accept": "application/json",
            "Authorization": f"Bearer {args.token}",
            "Content-Type": "application/json",
            "User-Agent": "nod-issuer/0.1",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            data = json.load(response)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", "replace")
        print(f"Nod returned HTTP {exc.code}: {detail}", file=sys.stderr)
        return 1
    except urllib.error.URLError as exc:
        print(f"Could not reach Nod: {exc.reason}", file=sys.stderr)
        return 1

    print(data.get("request_id", json.dumps(data)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
