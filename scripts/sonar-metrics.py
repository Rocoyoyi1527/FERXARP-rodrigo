#!/usr/bin/env python3
"""Save a token-free snapshot of measured SonarQube project results."""

import argparse
import json
import os
import sys
from urllib.parse import urlencode
from urllib.request import Request, urlopen


METRICS = (
    "bugs",
    "vulnerabilities",
    "security_hotspots",
    "code_smells",
    "coverage",
    "duplicated_lines_density",
    "sqale_index",
    "sqale_rating",
    "reliability_rating",
    "security_rating",
    "ncloc",
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("project")
    parser.add_argument("output")
    parser.add_argument("--host", default=os.getenv("SONAR_HOST_URL", "http://127.0.0.1:19000"))
    args = parser.parse_args()
    token = os.getenv("SONAR_TOKEN")
    if not token:
        parser.error("SONAR_TOKEN is required")

    def fetch(path: str, **params):
        suffix = "?" + urlencode(params) if params else ""
        request = Request(
            args.host.rstrip("/") + path + suffix,
            headers={"Authorization": f"Bearer {token}"},
        )
        with urlopen(request, timeout=30) as response:
            body = response.read().decode("utf-8")
            try:
                return json.loads(body)
            except json.JSONDecodeError:
                return body

    measures = fetch(
        "/api/measures/component",
        component=args.project,
        metricKeys=",".join(METRICS),
    )
    gate = fetch("/api/qualitygates/project_status", projectKey=args.project)
    issues = fetch("/api/issues/search", componentKeys=args.project, ps=500)
    hotspots = fetch("/api/hotspots/search", projectKey=args.project, ps=500)
    snapshot = {
        "project": args.project,
        "server_version": fetch("/api/server/version"),
        "quality_gate": gate.get("projectStatus"),
        "measures": {m["metric"]: m.get("value") for m in measures["component"]["measures"]},
        "issues_total": issues.get("total"),
        "issues": [
            {
                key: issue.get(key)
                for key in ("key", "rule", "type", "severity", "component", "line", "message", "status")
                if key in issue
            }
            for issue in issues.get("issues", [])
        ],
        "hotspots_total": hotspots.get("paging", {}).get("total"),
        "hotspots": [
            {
                key: hotspot.get(key)
                for key in ("key", "ruleKey", "component", "line", "message", "status", "vulnerabilityProbability")
                if key in hotspot
            }
            for hotspot in hotspots.get("hotspots", [])
        ],
    }
    with open(args.output, "w", encoding="utf-8") as destination:
        json.dump(snapshot, destination, ensure_ascii=False, indent=2)
        destination.write("\n")
    print(f"Saved SonarQube metrics for {args.project} without credentials")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"SonarQube metrics request failed: {error}", file=sys.stderr)
        raise SystemExit(1)
