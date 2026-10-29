#!/usr/bin/env python3
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Allowed IPv4 ranges: RFC 5737 TEST-NETs and loopback
ALLOWED_IPS = [
    re.compile(r"127(?:\.(?:25[0-5]|2[0-4]\d|1?\d?\d)){3}\b"),
    re.compile(r"192\.0\.2\.(?:25[0-5]|2[0-4]\d|1?\d?\d)\b"),
    re.compile(r"198\.51\.100\.(?:25[0-5]|2[0-4]\d|1?\d?\d)\b"),
    re.compile(r"203\.0\.113\.(?:25[0-5]|2[0-4]\d|1?\d?\d)\b"),
]
IPV4_RE = re.compile(
    r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b"
)

# For hostnames: disallow *.local except example.local
LOCAL_DOMAIN_RE = re.compile(r"\b(?!example\.local)[A-Za-z0-9-]+\.local\b", re.IGNORECASE)

# Heuristic hostnames like XX-XXXXX or XXX-XXX... (uppercase with dash + 3+)
# Hostname heuristic: UPPER-ALNUM with hyphen and at least one digit in the suffix (e.g., WS-01, DT-6HOP3)
HOSTNAME_HEURISTIC = re.compile(r"\b[A-Z]{2,8}-[A-Z0-9]*\d[A-Z0-9]*\b")

# MAC address patterns
MAC_RE = re.compile(r"\b([0-9A-Fa-f]{2}([-:])){5}[0-9A-Fa-f]{2}\b")

TEXT_EXTS = {
    ".rs", ".toml", ".md", ".yml", ".yaml", ".json", ".ps1", ".psd1", ".psm1",
    ".dart", ".proto", ".sh", ".bat", ".iss", ".txt", ".py", ".ts", ".tsx"
}

SKIP_DIRS = {".git", "target", "build", "dist", ".dart_tool", "node_modules", "installer/out"}
SKIP_FILES = {".gitignore"}

def is_allowed_ip(s: str) -> bool:
    return any(p.search(s) for p in ALLOWED_IPS)

def scan_file(path: Path):
    violations = []
    try:
        data = path.read_text(encoding="utf-8", errors="strict")
    except Exception:
        # Skip binary or unreadable files
        return violations

    # IP checks
    for m in IPV4_RE.finditer(data):
        if not is_allowed_ip(m.group(0)):
            violations.append((path, f"Non-doc IPv4 found: {m.group(0)}"))

    # .local domains
    for m in LOCAL_DOMAIN_RE.finditer(data):
        violations.append((path, f"Disallowed .local domain: {m.group(0)}"))

    # Heuristic hostnames
    for m in HOSTNAME_HEURISTIC.finditer(data):
        token = m.group(0)
        if re.fullmatch(r"(WS|PRN)-\d{2,3}", token) or token in {"RCU-01"}:
            continue
        violations.append((path, f"Suspicious hostname-like token: {token}"))

    # MACs
    for m in MAC_RE.finditer(data):
        violations.append((path, f"MAC address found: {m.group(0)}"))

    return violations

def main():
    root = ROOT
    violations = []
    for dirpath, dirnames, filenames in os.walk(root):
        dp = Path(dirpath)
        # prune
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS and not d.startswith(".git")]
        for fn in filenames:
            p = dp / fn
            if p.suffix.lower() not in TEXT_EXTS:
                # Best-effort: treat small, extensionless files as text
                if p.suffix == "" and p.stat().st_size < 512_000:
                    pass
                else:
                    continue
            if p.resolve() == Path(__file__).resolve() or p.name in SKIP_FILES:
                continue
            violations.extend(scan_file(p))
    if violations:
        print("Found potential site-specific content:")
        for path, msg in violations:
            print(f" - {path.relative_to(root)}: {msg}")
        print("\nFailing. Use placeholders (example.local, SERVER01, 192.0.2.x/198.51.100.x/203.0.113.x).", file=sys.stderr)
        sys.exit(2)
    print("OK: no site-specific strings detected.")

if __name__ == "__main__":
    main()

