"""Initialize the authorized fresh demo using official provision/seed mechanisms."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import urllib.request

ROOT = Path("/srv/ferxarp")
COMPOSE = ["bash", str(ROOT / "repo/deploy/eisenherz/compose.sh")]
env = dict(line.split("=", 1) for line in (ROOT / "deploy/.env").read_text().splitlines() if line and not line.startswith("#"))

def compose(*args, extra_env=None):
    return subprocess.check_output(COMPOSE + list(args), env={**os.environ, **(extra_env or {})}, text=True).strip()

def sql(query):
    return compose("exec", "-T", "postgres", "psql", "-U", "ferxarp", "-d", "ferxarp", "-At", "-c", query)

def fingerprints():
    return {table: sql(f"SELECT count(*) || ':' || md5(string_agg(to_jsonb(t)::text, '' ORDER BY id)) FROM {table} t") for table in ["users", "ngos", "donations", "donation_requests", "delivery_logs"]}

def request(path, payload=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request("http://127.0.0.1:18080" + path, data=None if payload is None else json.dumps(payload).encode(), headers=headers)
    with urllib.request.urlopen(req, timeout=120) as response:
        return json.load(response)

compose("run", "--rm", "--no-deps", "-e", "FERXARP_ADMIN_EMAIL", "-e", "FERXARP_ADMIN_PASSWORD", "backend", "/app/provision_admin", extra_env={key:env[key] for key in ["FERXARP_ADMIN_EMAIL", "FERXARP_ADMIN_PASSWORD"]})
login = request("/api/auth/login", {"email":env["FERXARP_ADMIN_EMAIL"], "password":env["FERXARP_ADMIN_PASSWORD"]})
token = login["token"]
source = (ROOT / "repo/backend/src/api/seed/data.rs").read_text()
company_source = source.split("pub const COMPANIES:")[1].split("pub const NGOS:")[0]
expected = {"companies":len(re.findall(r'"[^"\n]+@demo\.ferxarp\.invalid"', company_source)), "ngos":len(re.findall(r'email: "',source)), "donations":len(re.findall(r'title: "Demo · ',source))}
first = request("/api/seed/veracruz?include_ai=false", {}, token)
before = fingerprints()
second = request("/api/seed/veracruz?include_ai=false", {}, token)
after = fingerprints()
actual = json.loads(sql("SELECT json_build_object('companies',(SELECT count(*) FROM users WHERE role='empresa'),'ngos',(SELECT count(*) FROM ngos),'donations',(SELECT count(*) FROM donations))"))
assert actual == expected, {"expected":expected,"actual":actual}
assert before == after, "Second seed modified data"
assert sql("SELECT count(*) FROM donations WHERE title='Cajas de leche' OR id='f5d13322-a795-4ef3-bccc-a36771bd9c07'") == "0"
result = {"expected_from_fixture":expected,"actual_database":actual,"idempotent":before==after,"historical_donation_count":0,"first_seed":{k:v for k,v in first.items() if k.endswith("_seeded")},"second_seed":{k:v for k,v in second.items() if k.endswith("_seeded")},"map_points":len(request("/api/scanner/map-points",token=token)),"fingerprint_sha256":hashlib.sha256(json.dumps(after,sort_keys=True).encode()).hexdigest()}
(ROOT / "deploy/seed-validation.json").write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps(result,indent=2))
