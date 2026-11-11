#!/usr/bin/env python3
"""Publish the itcmdr-bin Arch recipe (PKGBUILD + .SRCINFO) of a published GitHub release to TAUR.

    python3 scripts/publish_taur.py --dry-run 1.4.0      # every check, then print the upload; uploads nothing
    python3 scripts/publish_taur.py 1.4.0                # upload with taurctl, then verify what TAUR serves
    python3 scripts/publish_taur.py --out DIR 1.4.0      # also keep the recipe archive in DIR (never overwritten)

A post-release step, after `scripts/release-local.sh publish`. TAUR takes recipes, not built packages: the
recipe downloads the release's linux-x86_64 tarballs of itcmdr-cli and itcmdr-admin. Only for a PUBLIC
repository: `gh` must report it PUBLIC and GitHub's API must answer an anonymous request with `private`
false and `visibility` public, both checked again just before the upload. Private, internal, unknown or a
failed check never uploads.

Refuses unless: vX.Y.Z is an annotated tag here and on origin at the same commit; the GitHub release vX.Y.Z
is published (not a draft) and has SHA256SUMS and both tarballs; the anonymously downloaded tarballs match
SHA256SUMS, hold only the expected files, and their binaries carry the build stamp of that version and the
tag's commit; every *.pkg.zst / *.pkg.tar.zst asset of the release has its non-empty .sig (Michael signs;
nothing here signs or reads keys); the recipe passes the repo's privacy scan; and .SRCINFO from
`makepkg --printsrcinfo` names exactly that version, those URLs and those checksums.

TAUR state: an existing itcmdr-bin with the same version and byte-identical PKGBUILD/.SRCINFO is already
published (nothing is uploaded); the same version with a different recipe, an older version than TAUR's, or
another owner is refused. Upload: `taurctl --server URL pkg upload ARCHIVE --org WeirdBrains -y`, using the
signed-in taurctl session (never printed). Afterwards TAUR's info and its served PKGBUILD and .SRCINFO must
match. Tools can be replaced for tests: ITCMDR_GH, ITCMDR_TAURCTL, ITCMDR_CURL, ITCMDR_MAKEPKG, ITCMDR_VERCMP.
"""
import argparse, hashlib, io, json, os, re, shutil, subprocess, sys, tarfile, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import check_pkg_sigs  # noqa: E402
import scan_repo  # noqa: E402

PKG = "itcmdr-bin"
BINS = ("itcmdr-cli", "itcmdr-admin")
ORG = "WeirdBrains"
SERVER = "https://pkg.taur.weirdbrains.com"
TEMPLATE = ROOT / "packaging" / "arch" / PKG / "PKGBUILD"
SEMVER = re.compile(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)")
STAMP = re.compile(rb"ITCMDR-BUILD v=(\S+) commit=([0-9a-f]{40}) help=[0-9a-f]{16} END")


class Refused(Exception):
    pass


def tool(name):
    return os.environ.get(f"ITCMDR_{name.upper()}", name)


def run(cmd, *, cwd=None, check=True):
    p = subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True)
    if check and p.returncode != 0:
        raise Refused(f"{' '.join(map(str, cmd[:4]))} failed: {(p.stderr or p.stdout).strip()[:300]}")
    return p


def git(*args):
    return run(["git", *args]).stdout.strip()


def tarball(ver, b):
    return f"{b}-{ver}-linux-x86_64.tar.gz"


def github_slug():
    url = git("config", "--get", "remote.origin.url")
    m = re.fullmatch(r"(?:git@github\.com:|ssh://git@github\.com/|https://github\.com/)([\w.-]+)/([\w.-]+?)(?:\.git)?/?", url)
    if not m:
        raise Refused(f"origin ({url}) is not a GitHub repository")
    return f"{m.group(1)}/{m.group(2)}"


def fetch(url, dest):
    """Anonymous HTTPS download: no netrc, no curlrc, no token."""
    run([tool("curl"), "-q", "--proto", "=https", "--netrc-file", os.devnull, "-fsSL", "-o", str(dest), url])
    return dest


def confirm_public(slug, work):
    """Both must hold: gh says PUBLIC, and GitHub's API answers public to an anonymous request."""
    p = run([tool("gh"), "repo", "view", slug, "--json", "visibility", "--jq", ".visibility"], check=False)
    vis = p.stdout.strip()
    if p.returncode != 0 or vis != "PUBLIC":
        raise Refused(f"{slug} is not confirmed PUBLIC (gh: {vis or p.stderr.strip()[:200] or 'no answer'}); nothing is uploaded to TAUR")
    probe = Path(work) / "public-probe.json"
    try:
        meta = json.loads(fetch(f"https://api.github.com/repos/{slug}", probe).read_text())
    except (Refused, ValueError) as e:
        raise Refused(f"{slug} is not visible anonymously; nothing is uploaded to TAUR ({e})")
    finally:
        probe.unlink(missing_ok=True)
    if meta.get("full_name", "").lower() != slug.lower() or meta.get("private") is not False or meta.get("visibility") != "public":
        raise Refused(f"anonymous GitHub API does not report {slug} public; nothing is uploaded to TAUR")


def check_tag(ver):
    tag = f"v{ver}"
    if run(["git", "cat-file", "-t", f"refs/tags/{tag}"], check=False).stdout.strip() != "tag":
        raise Refused(f"{tag} is not an annotated tag here")
    commit = git("rev-parse", f"{tag}^{{commit}}")
    remote = git("ls-remote", "origin", f"refs/tags/{tag}^{{}}").split()
    if not remote or remote[0] != commit:
        raise Refused(f"origin's {tag} is not at {commit[:12]}")
    return commit


def check_release(slug, ver):
    p = run([tool("gh"), "release", "view", f"v{ver}", "-R", slug, "--json", "tagName,isDraft,assets"])
    rel = json.loads(p.stdout)
    if rel.get("tagName") != f"v{ver}" or rel.get("isDraft"):
        raise Refused(f"v{ver} is not a published GitHub release")
    names = {a["name"]: a for a in rel.get("assets", [])}
    need = ["SHA256SUMS"] + [tarball(ver, b) for b in BINS]
    missing = [n for n in need if n not in names]
    if missing:
        raise Refused(f"release v{ver} lacks {', '.join(missing)}")
    # Hard rule: a pacman package asset needs its matching non-empty .sig asset.
    for n in names:
        if n.endswith((".pkg.zst", ".pkg.tar.zst")):
            sig = names.get(n + ".sig")
            if not sig or not sig.get("size"):
                raise Refused(f"release asset {n} has no non-empty {n}.sig (Michael signs packages; nothing here signs)")
    return names


def verify_tarballs(slug, ver, commit, d):
    base = f"https://github.com/{slug}/releases/download/v{ver}"
    sums = {}
    for line in fetch(f"{base}/SHA256SUMS", d / "SHA256SUMS").read_text().splitlines():
        if line.strip():
            h, n = line.split(None, 1)
            sums[n.strip().lstrip("*")] = h
    out = {}
    for b in BINS:
        n = tarball(ver, b)
        if n not in sums:
            raise Refused(f"SHA256SUMS has no {n}")
        f = fetch(f"{base}/{n}", d / n)
        h = hashlib.sha256(f.read_bytes()).hexdigest()
        if h != sums[n]:
            raise Refused(f"{n} does not match SHA256SUMS")
        top = f"{b}-{ver}-linux-x86_64"
        with tarfile.open(f) as t:
            members = t.getmembers()
            want = {top, f"{top}/README.txt", f"{top}/{b}"}
            got = {m.name.rstrip("/") for m in members}
            if got != want or any(m.issym() or m.islnk() for m in members):
                raise Refused(f"{n} holds {sorted(got)}, expected {sorted(want)}")
            data = t.extractfile(f"{top}/{b}").read()
        stamps = set(STAMP.findall(data))
        if stamps != {(ver.encode(), commit.encode())}:
            raise Refused(f"{n}: {b} is not stamped v={ver} commit={commit[:12]} ({stamps or 'no stamp'})")
        out[n] = h
    # The same rule over everything downloaded (no package files are expected).
    bad = check_pkg_sigs.pair_problems(d)
    if bad:
        raise Refused(f"unsigned package files: {bad}")
    return out


def render(ver, slug, sums):
    s = TEMPLATE.read_text()
    s, n1 = re.subn(r"^pkgver=.*$", f"pkgver={ver}", s, count=1, flags=re.M)
    s, n2 = re.subn(r"^sha256sums=\('SKIP'\n\s+'SKIP'\)$",
                    "sha256sums=('{}'\n            '{}')".format(*(sums[tarball(ver, b)] for b in BINS)), s, count=1, flags=re.M)
    if (n1, n2) != (1, 1) or f"url='https://github.com/{slug}'" not in s or "SKIP" in s:
        raise Refused("packaging/arch/itcmdr-bin/PKGBUILD is not the expected template")
    if re.search(r"^pkgrel=1$", s, re.M) is None or re.search(rf"^pkgname={PKG}$", s, re.M) is None:
        raise Refused("PKGBUILD must be pkgname=itcmdr-bin, pkgrel=1")
    return s


def srcinfo(recipe, ver, slug, sums):
    p = run([tool("makepkg"), "--printsrcinfo"], cwd=recipe)
    text = p.stdout
    f = {}
    for line in text.splitlines():
        if " = " in line:
            k, v = line.strip().split(" = ", 1)
            f.setdefault(k, []).append(v)
    base = f"https://github.com/{slug}/releases/download/v{ver}"
    want = {
        "pkgbase": [PKG], "pkgname": [PKG], "pkgver": [ver], "pkgrel": ["1"], "arch": ["x86_64"],
        "source": [f"{base}/{tarball(ver, b)}" for b in BINS],
        "sha256sums": [sums[tarball(ver, b)] for b in BINS],
    }
    for k, v in want.items():
        if f.get(k) != v:
            raise Refused(f".SRCINFO {k} is {f.get(k)}, expected {v}")
    return text


def archive(recipe_files):
    """Deterministic tar.gz with itcmdr-bin/PKGBUILD and itcmdr-bin/.SRCINFO only."""
    buf = io.BytesIO()
    import gzip
    with gzip.GzipFile(filename="", mode="wb", fileobj=buf, mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.USTAR_FORMAT) as t:
            for name, data in recipe_files:
                ti = tarfile.TarInfo(f"{PKG}/{name}")
                ti.size, ti.mode, ti.mtime, ti.uid, ti.gid = len(data), 0o644, 0, 0, 0
                t.addfile(ti, io.BytesIO(data))
    return buf.getvalue()


def taur_info(server):
    p = run([tool("taurctl"), "--server", server, "--json", "info", PKG])
    res = json.loads(p.stdout).get("results", [])
    return res[0] if res else None


def served(server, name, d):
    return fetch(f"{server}/cgit/aur.git/plain/{name}?h={PKG}", d / f"served-{name.lstrip('.')}").read_bytes()


def publish(ver, *, dry_run=False, out=None, server=SERVER, org=ORG, log=print):
    if not SEMVER.fullmatch(ver):
        raise Refused(f"VERSION must be X.Y.Z, not {ver!r}")
    if not server.startswith("https://"):
        raise Refused("the TAUR server must be an https:// URL")
    slug = github_slug()
    with tempfile.TemporaryDirectory(prefix="itcmdr-taur-") as w:
        work = Path(w)
        confirm_public(slug, work)
        commit = check_tag(ver)
        check_release(slug, ver)
        dl = work / "download"; dl.mkdir()
        sums = verify_tarballs(slug, ver, commit, dl)
        recipe = work / PKG; recipe.mkdir()
        pkgbuild = render(ver, slug, sums)
        (recipe / "PKGBUILD").write_text(pkgbuild)
        info_text = srcinfo(recipe, ver, slug, sums)
        (recipe / ".SRCINFO").write_text(info_text)
        bad = [v for p in (recipe / "PKGBUILD", recipe / ".SRCINFO") for v in scan_repo.scan_file(p)]
        if bad:
            raise Refused(f"privacy scan: {bad[0][1]}")
        files = [("PKGBUILD", pkgbuild.encode()), (".SRCINFO", info_text.encode())]
        data = archive(files)
        name = f"{PKG}-{ver}-1.src.tar.gz"
        path = work / name
        path.write_bytes(data)
        if out:
            dest = Path(out) / name
            if dest.exists() and dest.read_bytes() != data:
                raise Refused(f"{dest} exists with other content; not overwriting it")
            Path(out).mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
            log(f"recipe archive: {dest} (sha256 {hashlib.sha256(data).hexdigest()})")

        cur = taur_info(server)
        full = f"{ver}-1"
        if cur:
            if cur.get("PackageBase") != PKG or cur.get("Organization") != org:
                raise Refused(f"TAUR {PKG} belongs to {cur.get('Organization') or cur.get('Maintainer')}, not {org}")
            if cur.get("Version") == full:
                same = served(server, "PKGBUILD", work) == files[0][1] and served(server, ".SRCINFO", work) == files[1][1]
                if same:
                    log(f"TAUR already has {PKG} {full} with this exact recipe; nothing to upload")
                    return "unchanged"
                raise Refused(f"TAUR has {PKG} {full} with a different recipe; not replacing it (raise pkgrel by hand)")
            if run([tool("vercmp"), full, cur.get("Version", "")]).stdout.strip() != "1":
                raise Refused(f"TAUR has {PKG} {cur.get('Version')}, which is not older than {full}")
        cmd = [tool("taurctl"), "--server", server, "pkg", "upload", str(path), "--org", org, "-y"]
        log(f"{slug} v{ver} ({commit[:12]}) -> TAUR {server} {PKG} {full} (org {org}); TAUR now: {cur.get('Version') if cur else 'none'}")
        if dry_run:
            log("dry run: would run: " + " ".join(cmd))
            log("dry run: nothing was uploaded")
            return "dry-run"
        confirm_public(slug, work)  # again, immediately before the upload
        run(cmd)
        after = taur_info(server)
        if not after or after.get("Version") != full or after.get("Organization") != org:
            raise Refused(f"after the upload TAUR reports {after and after.get('Version')}; check {server}/packages/{PKG}")
        if served(server, "PKGBUILD", work) != files[0][1] or served(server, ".SRCINFO", work) != files[1][1]:
            raise Refused("TAUR serves a different PKGBUILD/.SRCINFO than was uploaded")
        log(f"published {PKG} {full} to {server} (org {org}); served PKGBUILD and .SRCINFO verified")
        return "published"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("version")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--out")
    ap.add_argument("--server", default=SERVER)
    a = ap.parse_args(argv)
    try:
        publish(a.version.removeprefix("v"), dry_run=a.dry_run, out=a.out, server=a.server.rstrip("/"))
    except Refused as e:
        print(f"publish_taur: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
