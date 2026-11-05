#!/usr/bin/env python3
"""v1.3.1 release guard: every release asset must be built from the tagged commit.

    python3 scripts/check_stamps.py <dist dir> <commit> [--stamps <file>] [--docs <dir>]   # exit 1 on any mismatch
    python3 scripts/check_stamps.py --help-hash                             # print the docs/help hash

Each binary carries "ITCMDR-BUILD v=<version> commit=<sha> help=<hash> END" (help/build.rs). The
tar.gz/zip/apk assets are unpacked and searched. Windows installers are compressed by Inno Setup,
so for *.exe the stamps reported by the VM build (rel.ps1 writes `<exe>: <stamp>` lines into a
stamps file) are used when the installer itself shows none. Every asset needs at least one stamp,
and every stamp must name the expected commit and the current docs' help hash.
"""
import re, sys, tarfile, zipfile, io
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STAMP = re.compile(rb"ITCMDR-BUILD v=(\S+) commit=([0-9a-zA-Z]+) help=([0-9a-f]{16}) END")


def help_hash(docs: Path = ROOT / "docs") -> str:
    """FNV-1a 64 over (topic id, NUL, file bytes, NUL), topics sorted; same as help/build.rs."""
    items = []
    for f in docs.rglob("*.md"):
        rel = f.relative_to(docs)
        if rel.parts[0] == "historical":
            continue
        tid = rel.with_suffix("").as_posix().lower()
        items.append(("index" if tid == "readme" else tid, f))
    h = 0xCBF29CE484222325
    for tid, f in sorted(items):
        for b in tid.encode() + b"\0" + f.read_bytes() + b"\0":
            h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def stamps_in(path: Path):
    """All stamps (version, commit, help) inside one asset."""
    found = set()
    def scan(data: bytes):
        for m in STAMP.finditer(data):
            found.add(tuple(x.decode() for x in m.groups()))
    name = path.name
    if name.endswith(".tar.gz"):
        with tarfile.open(path) as t:
            for m in t.getmembers():
                if m.isfile():
                    scan(t.extractfile(m).read())
    elif name.endswith((".zip", ".apk")):
        with zipfile.ZipFile(path) as z:
            for n in z.namelist():
                scan(z.read(n))
    else:
        scan(path.read_bytes())
    return found


def check(dist: Path, commit: str, hh: str, stamps_file: Path | None = None):
    """Returns a list of problems (empty = all good)."""
    side = {}
    if stamps_file and stamps_file.exists():
        for line in stamps_file.read_text().splitlines():
            if ":" in line:
                exe, rest = line.split(":", 1)
                side.setdefault(exe.strip(), set()).update(
                    tuple(x.decode() for x in m.groups()) for m in STAMP.finditer(rest.encode()))
    bad = []
    for a in sorted(dist.iterdir()):
        if a.name == "SHA256SUMS" or not a.is_file():
            continue
        s = stamps_in(a)
        if not s and a.suffix == ".exe":
            s = side.get(a.name, set())
        if not s:
            bad.append(f"{a.name}: no build stamp")
            continue
        for v, c, h in sorted(s):
            if c != commit:
                bad.append(f"{a.name}: built from {c[:12]}, tag is {commit[:12]}")
            if h != hh:
                bad.append(f"{a.name}: help {h} != docs {hh}")
        print(f"stamp {a.name}: {len(s)} ok" if not any(x.startswith(a.name + ':') for x in bad) else f"stamp {a.name}: MISMATCH")
    return bad


def main(argv):
    if "--help-hash" in argv:
        print(help_hash()); return 0
    if len(argv) < 3:
        print(__doc__); return 2
    sf = Path(argv[argv.index("--stamps") + 1]) if "--stamps" in argv else None
    docs = Path(argv[argv.index("--docs") + 1]) if "--docs" in argv else ROOT / "docs"
    bad = check(Path(argv[1]), argv[2], help_hash(docs), sf)
    for b in bad:
        print("GUARD:", b, file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
