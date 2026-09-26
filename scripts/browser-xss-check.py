#!/usr/bin/env python3
"""Exercise stored map popup data in a local headless Chromium session."""

import argparse
import json
import os
import sys
import time
from urllib.error import HTTPError
from urllib.parse import urlparse
from urllib.request import Request, urlopen


def request_json(url, method="GET", payload=None):
    body = json.dumps(payload).encode() if payload is not None else None
    request = Request(
        url,
        method=method,
        data=body,
        headers={"Content-Type": "application/json"} if body is not None else {},
    )
    try:
        with urlopen(request, timeout=30) as response:
            return json.load(response)
    except HTTPError as error:
        return json.load(error)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--phase", choices=("before", "after"), required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    frontend = os.getenv("FRONTEND_URL", "http://localhost:13004").rstrip("/")
    backend = os.getenv("BACKEND_URL", "http://127.0.0.1:18004").rstrip("/")
    for url in (frontend, backend):
        parsed = urlparse(url)
        if parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "localhost"):
            parser.error("Only local HTTP targets are permitted")
    email = os.getenv("FERXARP_ADMIN_EMAIL")
    password = os.getenv("FERXARP_ADMIN_PASSWORD")
    if not email or not password:
        parser.error("Disposable Admin credentials are required")
    login = request_json(
        backend + "/api/auth/login",
        "POST",
        {"email": email, "password": password},
    )
    token = login.get("token")
    if not token:
        raise RuntimeError("Admin login failed")

    webdriver = "http://127.0.0.1:19515"
    session = request_json(
        webdriver + "/session",
        "POST",
        {"capabilities": {"alwaysMatch": {
            "browserName": "chrome",
            "goog:chromeOptions": {"args": [
                "--headless=new",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--disable-background-networking",
                "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost",
                "--user-data-dir=/tmp/ferxarp-act03-chrome",
            ]},
        }}},
    )
    session_id = session.get("value", {}).get("sessionId")
    if not session_id:
        raise RuntimeError(f"ChromeDriver session failed: {session.get('value', {}).get('error')}")
    prefix = f"{webdriver}/session/{session_id}"

    def navigate(url):
        return request_json(prefix + "/url", "POST", {"url": url})

    def execute(script, *arguments):
        return request_json(prefix + "/execute/sync", "POST", {"script": script, "args": arguments})

    try:
        navigate(frontend + "/login")
        execute("localStorage.setItem('fexarp_token', arguments[0]);", token)
        navigate(frontend + "/map")
        marker_count = 0
        for _ in range(60):
            result = execute("return document.querySelectorAll('.leaflet-marker-icon').length;")
            marker_count = result.get("value") or 0
            if marker_count >= 2:
                break
            time.sleep(0.5)
        if marker_count < 2:
            raise RuntimeError("Authenticated map markers did not render")

        alert_text = None
        raw_html_image_seen = False
        for index in range(marker_count):
            execute(
                "document.querySelectorAll('.leaflet-marker-icon')[arguments[0]].click();",
                index,
            )
            time.sleep(0.15)
            alert = request_json(prefix + "/alert/text")
            if "error" not in alert.get("value", {}):
                alert_text = alert.get("value")
                request_json(prefix + "/alert/dismiss", "POST", {})
                raw_html_image_seen = True
                break
            image = execute("return !!document.querySelector('.leaflet-popup-content img[onerror]');")
            raw_html_image_seen = raw_html_image_seen or image.get("value") is True

        report = {
            "phase": args.phase,
            "frontend_url": frontend,
            "backend_url": backend,
            "browser": "Chromium headless via ChromeDriver",
            "markers_rendered": marker_count,
            "stored_payload": "<img src=x onerror=alert(1)>",
            "alert_observed": alert_text is not None,
            "alert_text": alert_text,
            "raw_img_onerror_in_popup": raw_html_image_seen,
        }
        with open(args.output, "w", encoding="utf-8") as destination:
            json.dump(report, destination, ensure_ascii=False, indent=2)
            destination.write("\n")
        print(f"Stored map payload executed in Chromium: {report['alert_observed']}")
        if report["alert_observed"] != (args.phase == "before"):
            raise RuntimeError("Browser XSS result differs from expected before/after state")
    finally:
        request_json(prefix, "DELETE")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"Browser XSS check failed: {error}", file=sys.stderr)
        raise SystemExit(1)
