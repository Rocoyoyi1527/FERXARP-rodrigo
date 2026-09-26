#!/usr/bin/env python3
"""Controlled SQLi/XSS probes against the disposable ACT-03 backend only."""

import argparse
import json
import os
import re
import sys
from urllib.error import HTTPError
from urllib.parse import quote, urlparse
from urllib.request import Request, urlopen


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--phase", choices=("before", "after"), required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--sample-id", help="Unique lowercase label for disposable test rows")
    args = parser.parse_args()
    sample_id = args.sample_id or args.phase
    if not re.fullmatch(r"[a-z0-9-]+", sample_id):
        parser.error("--sample-id must contain only lowercase letters, digits and hyphens")
    base = os.getenv("BACKEND_URL", "http://127.0.0.1:18004").rstrip("/")
    parsed = urlparse(base)
    if parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "localhost"):
        parser.error("BACKEND_URL must point to local HTTP only")
    admin_email = os.getenv("FERXARP_ADMIN_EMAIL")
    admin_password = os.getenv("FERXARP_ADMIN_PASSWORD")
    if not admin_email or not admin_password:
        parser.error("FERXARP_ADMIN_EMAIL and FERXARP_ADMIN_PASSWORD are required")

    records = []

    def call(label, method, path, payload=None, token=None, probe=None):
        body = json.dumps(payload).encode() if payload is not None else None
        headers = {"Content-Type": "application/json"} if body is not None else {}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        request = Request(base + path, data=body, method=method, headers=headers)
        try:
            with urlopen(request, timeout=15) as response:
                status, raw = response.status, response.read()
        except HTTPError as error:
            status, raw = error.code, error.read()
        try:
            data = json.loads(raw)
        except (json.JSONDecodeError, UnicodeDecodeError):
            data = None
        records.append({"id": label, "method": method, "path": path, "probe": probe, "status": status})
        return status, data

    status, login = call(
        "admin_login", "POST", "/api/auth/login",
        {"email": admin_email, "password": admin_password},
    )
    if status != 200 or not isinstance(login, dict) or not login.get("token"):
        raise RuntimeError("Disposable Admin login failed")
    token = login["token"]

    injection = "' OR '1'='1"
    status, result = call(
        "login_sqli", "POST", "/api/auth/login",
        {"email": injection, "password": "invalid-test-password"}, probe=injection,
    )
    if status != 401 or isinstance(result, dict) and result.get("token"):
        raise RuntimeError("Login injection was not rejected")

    literal_email = f"sqli-{sample_id}.{injection}@example.invalid"
    status, _ = call(
        "register_literal_quote", "POST", "/api/auth/register",
        {"email": literal_email, "password": "act03-regular-password", "role": "empresa"},
        probe=literal_email,
    )
    if status not in (201, 409):
        raise RuntimeError(f"Quoted registration input failed with HTTP {status}")
    status, literal_login = call(
        "login_literal_quote", "POST", "/api/auth/login",
        {"email": literal_email, "password": "act03-regular-password"},
    )
    if status != 200 or not isinstance(literal_login, dict) or not literal_login.get("token"):
        raise RuntimeError("Literal quoted email could not log in")

    xss = "<img src=x onerror=alert(1)>"
    xss_email = f"xss-{sample_id}.{xss}@example.invalid"
    status, _ = call(
        "register_ngo_html", "POST", "/api/auth/register",
        {"email": xss_email, "password": "act03-regular-password", "role": "ong"},
        probe=xss,
    )
    if status not in (201, 409):
        raise RuntimeError(f"ONG HTML test data failed with HTTP {status}")
    status, points = call("map_points_authenticated", "GET", "/api/scanner/map-points", token=token)
    stored_name = f"xss-{sample_id}.{xss}"
    name_returned = status == 200 and isinstance(points, list) and any(
        point.get("name") == stored_name for point in points
    )
    records.append({"id": "ngo_name_stored_and_returned", "probe": xss, "observed": name_returned})
    if not name_returned:
        raise RuntimeError("Stored ONG name was not returned to the map API")

    status, donation = call(
        "donation_literal_quote", "POST", "/api/donations",
        {"title": injection, "description": "<script>alert(1)</script>", "quantity": 1},
        token=token, probe=injection,
    )
    if status != 201 or not isinstance(donation, dict) or not donation.get("id"):
        raise RuntimeError(f"Controlled test donation creation failed with HTTP {status}")
    donation_id = donation["id"]
    status, items = call("donations_authenticated", "GET", "/api/donations", token=token)
    literal_returned = status == 200 and isinstance(items, list) and any(
        item.get("id") == donation_id and item.get("title") == injection for item in items
    )
    records.append({"id": "donation_literal_returned", "probe": injection, "observed": literal_returned})
    if not literal_returned:
        raise RuntimeError("Donation title was not returned literally")

    call(
        "donation_id_sqli", "GET",
        f"/api/donations/{quote(injection, safe='')}/matches?include_ai=false",
        token=token, probe=injection,
    )
    call(
        "match_filter_sqli", "GET",
        f"/api/donations/{donation_id}/matches?include_ai={quote(injection, safe='')}",
        token=token, probe=injection,
    )
    call(
        "scanner_uuid_sqli", "POST", "/api/scanner/scan",
        {"donation_id": injection, "action": "salida"}, token=token, probe=injection,
    )
    call(
        "tracking_id_sqli", "GET", f"/api/scanner/tracking/{quote(injection, safe='')}",
        token=token, probe=injection,
    )
    call(
        "seed_unauthenticated", "POST", "/api/seed/veracruz?include_ai=false",
        probe="No JWT; no mutation expected",
    )
    report = {"phase": args.phase, "sample_id": sample_id, "backend_url": base, "checks": records}
    with open(args.output, "w", encoding="utf-8") as destination:
        json.dump(report, destination, ensure_ascii=False, indent=2)
        destination.write("\n")
    print(f"Saved {len(records)} controlled HTTP checks without JWTs or passwords")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"Security HTTP check failed: {error}", file=sys.stderr)
        raise SystemExit(1)
