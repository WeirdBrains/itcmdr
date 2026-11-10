#!/usr/bin/env python3
"""Release guard: every pacman package asset has its detached signature next to it.

    python3 scripts/check_pkg_sigs.py <dist dir>     # exit 1 when a package is unsigned or a signature is bad

Every `*.pkg.zst` / `*.pkg.tar.zst` file needs `<exact file name>.sig` (standard pacman detached
signature, e.g. foo.pkg.tar.zst -> foo.pkg.tar.zst.sig) that exists and is non-empty. When gpg is
installed and knows the signing key, the signature must also verify (a bad signature fails). When gpg
is missing or has no public key for it, the name pairing is still required.

Signatures are never made here or on the build machines (alien-blue, the Mac, the Windows VM):
Michael signs the packages himself with a key that never lives on them. Used by
`scripts/release-local.sh check` and `publish`, and by scripts/release.py when dist/<version> exists.
"""
import shutil
import subprocess
import sys
from pathlib import Path

PKG_SUFFIXES = (".pkg.tar.zst", ".pkg.zst")


def is_package(name: str) -> bool:
    return name.endswith(PKG_SUFFIXES)


def pair_problems(dist: Path):
    """Pairing rule only: [(package name, problem)] for packages without a matching non-empty .sig."""
    bad = []
    for p in sorted(dist.iterdir()):
        if not p.is_file() or not is_package(p.name):
            continue
        sig = p.with_name(p.name + ".sig")
        if not sig.is_file():
            bad.append((p.name, f"missing signature {sig.name}"))
        elif sig.stat().st_size == 0:
            bad.append((p.name, f"empty signature {sig.name}"))
    return bad


def gpg_verify(pkg: Path, sig: Path, gpg: str):
    """'good', 'nokey' (cannot verify: no public key/keyring) or 'bad: <why>'."""
    r = subprocess.run([gpg, "--batch", "--status-fd", "1", "--verify", str(sig), str(pkg)],
                       capture_output=True, text=True)
    status = [l.split()[1] for l in r.stdout.splitlines() if l.startswith("[GNUPG:] ") and len(l.split()) > 1]
    if r.returncode == 0 and "GOODSIG" in status and "VALIDSIG" in status:
        return "good"
    if "BADSIG" in status:
        return "bad: BADSIG"
    if "NO_PUBKEY" in status and "BADSIG" not in status:
        return "nokey"
    return "bad: " + ((r.stderr or r.stdout).strip().splitlines() or ["gpg could not verify"])[-1]


def check(dist: Path, gpg: str | None = None, verify: bool = True):
    """All problems (empty = publishable). gpg=None looks gpg up on PATH."""
    bad = [f"{n}: {why}" for n, why in pair_problems(dist)]
    if bad or not verify:
        return bad
    gpg = gpg or shutil.which("gpg")
    for p in sorted(dist.iterdir()):
        if not p.is_file() or not is_package(p.name):
            continue
        sig = p.with_name(p.name + ".sig")
        if not gpg:
            print(f"sig {p.name}: paired (gpg not installed, not verified)")
            continue
        r = gpg_verify(p, sig, gpg)
        if r == "good":
            print(f"sig {p.name}: verified")
        elif r == "nokey":
            print(f"sig {p.name}: paired (no public key in the keyring, not verified)")
        else:
            bad.append(f"{p.name}: signature does not verify ({r[5:]})")
    return bad


def main(argv):
    if len(argv) != 2 or argv[1] in ("-h", "--help"):
        print(__doc__)
        return 2
    dist = Path(argv[1])
    if not dist.is_dir():
        print(f"GUARD: {dist} is not a directory", file=sys.stderr)
        return 1
    bad = check(dist)
    for b in bad:
        print("GUARD:", b, file=sys.stderr)
    if bad:
        print("GUARD: unsigned or badly signed packages are not publishable (Michael signs them first)", file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
