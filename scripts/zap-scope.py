"""Seed only local FERXARP routes into a ZAP packaged scan."""

from urllib.parse import urlparse


def zap_started(zap, target):
    parsed = urlparse(target)
    if parsed.hostname == "frontend" and parsed.port == 3000:
        paths = ("/", "/login", "/register", "/dashboard", "/shipments", "/map")
    elif parsed.hostname == "backend" and parsed.port == 8000:
        paths = (
            "/health",
            "/api/auth/login",
            "/api/auth/register",
            "/api/auth/me",
            "/api/auth/ngos",
            "/api/donations",
            "/api/donations/feed",
            "/api/donations/shipments",
            "/api/scanner/map-points",
            "/api/scanner/tracking/00000000-0000-0000-0000-000000000000",
            "/api/seed/veracruz",
        )
    else:
        raise ValueError("ZAP target must be an internal FERXARP frontend or backend")

    base = f"{parsed.scheme}://{parsed.netloc}"
    for path in paths:
        zap.urlopen(base + path)
    return zap, target


def zap_active_scan(zap, target, policy):
    # The environment is disposable, but keep active checks bounded.
    zap.ascan.set_option_max_scan_duration_in_mins(8)
    zap.ascan.set_option_max_rule_duration_in_mins(2)
    return zap, target, policy
