#!/usr/bin/env python3
"""Cut an itcmdr release: bump the product version, move the changelog, commit, tag and push.

    python3 scripts/release.py "commit message"            # minor bump (default): 1.3.8 -> 1.4.0
    python3 scripts/release.py --patch "commit message"    # 1.3.8 -> 1.3.9
    python3 scripts/release.py --major "commit message"    # 1.3.8 -> 2.0.0
    python3 scripts/release.py --dry-run --patch "msg"     # print what would change, change nothing
    python3 scripts/release.py --no-push "msg"             # commit and tag locally only

For actual releases only. Nothing in CI, git hooks or an ordinary push to main runs this; a push of
ordinary commits to main is not a release. It never builds, signs or publishes anything: build every
asset from the new tag afterwards as docs/RELEASE-LOCAL.md describes.

Steps (fails closed, nothing is pushed on any error):
  1. Checks: clean tree, on main, not behind origin/main, tag vX.Y.Z absent locally and on origin,
     notes under "## Unreleased" in CHANGELOG.md, and every *.pkg.zst / *.pkg.tar.zst already in
     dist/X.Y.Z has its matching pacman .sig (scripts/check_pkg_sigs.py; Michael signs them).
  2. Bumps every product version (the set the v1.3.8 release commit touched): the Rust product crates
     and their Cargo.lock entries, both apps' pubspec.yaml (X.Y.Z+N, build number + 1),
     site_jaspr/pubspec.yaml, $script:AgentVersion, installer/common.iss. Not itcmdr-common,
     common/derive, itcmdr-plugin or tools/sample-data-gen.
  3. CHANGELOG.md: "## Unreleased" notes move to "## vX.Y.Z (YYYY-MM-DD)" (America/New_York date);
     an empty "## Unreleased" stays on top. docs/HANDOVER.md latest tag, a PLAN.md row.
  4. scripts/gen-help.py (embedded help must match the tag), then --check.
  5. One commit with the message (version prefixed when missing), annotated tag vX.Y.Z,
     git push --follow-tags <remote> main.
"""
import argparse
import datetime
import re
import subprocess
import sys
from pathlib import Path

try:
    from zoneinfo import ZoneInfo
except ImportError:  # pragma: no cover
    ZoneInfo = None

ROOT = Path(__file__).resolve().parent.parent
BRANCH = "main"
TZ = "America/New_York"

# Product crates (Cargo.toml [package] version). Same set as the v1.3.8 release commit.
CRATES = [
    "server", "watchdog", "agent-rs", "keys", "agentlink", "adminsvr", "admin-cli", "help",
    "adminlink", "admin-app/rust", "app/rust",
]
LOCKS = ["Cargo.lock", "app/rust/Cargo.lock", "admin-app/rust/Cargo.lock"]
APP_PUBSPECS = ["app/pubspec.yaml", "admin-app/pubspec.yaml"]
SITE_PUBSPEC = "site_jaspr/pubspec.yaml"
AGENT_PS1 = "agent/Invoke-ItcmdrAgent.ps1"
INSTALLER = "installer/common.iss"
CHANGELOG = "CHANGELOG.md"
HANDOVER = "docs/HANDOVER.md"
PLAN = "PLAN.md"
GENERATED = ["app/lib/help_topics.g.dart", "admin-app/lib/help_topics.g.dart"]

SEMVER = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


class ReleaseError(Exception):
    pass


# --- pure helpers (unit-tested in scripts/test_release.py) ---------------------------------------

def parse_version(v):
    m = SEMVER.match(v.strip())
    if not m:
        raise ReleaseError(f"not a X.Y.Z version: {v!r}")
    return tuple(int(x) for x in m.groups())


def bump(version, part):
    """1.3.8 -> patch 1.3.9, minor 1.4.0, major 2.0.0 (lower parts reset)."""
    ma, mi, pa = parse_version(version)
    if part == "major":
        return f"{ma + 1}.0.0"
    if part == "minor":
        return f"{ma}.{mi + 1}.0"
    if part == "patch":
        return f"{ma}.{mi}.{pa + 1}"
    raise ReleaseError(f"unknown bump {part!r}")


def _sub_once(pattern, repl, text, what, flags=re.M):
    new, n = re.subn(pattern, repl, text, count=1, flags=flags)
    if n != 1:
        raise ReleaseError(f"{what}: expected pattern not found ({pattern})")
    return new


def cargo_package(text):
    """(name, version) from the [package] table of a Cargo.toml."""
    m = re.search(r"^\[package\]\s*\n(.*?)(?=^\[|\Z)", text, re.S | re.M)
    if not m:
        raise ReleaseError("no [package] table")
    body = m.group(1)
    name = re.search(r'^name\s*=\s*"([^"]+)"', body, re.M)
    ver = re.search(r'^version\s*=\s*"([^"]+)"', body, re.M)
    if not name or not ver:
        raise ReleaseError("[package] has no name or version")
    return name.group(1), ver.group(1)


def bump_cargo_toml(text, old, new, what):
    m = re.search(r"^\[package\]\s*\n(.*?)(?=^\[|\Z)", text, re.S | re.M)
    if not m:
        raise ReleaseError(f"{what}: no [package] table")
    body = m.group(1)
    nb = _sub_once(r'^version\s*=\s*"' + re.escape(old) + r'"', f'version = "{new}"', body, what)
    return text[: m.start(1)] + nb + text[m.end(1):]


def bump_lock(text, names, old, new):
    """Bump the path packages `names` at version `old` in a Cargo.lock; returns (text, bumped names)."""
    done = []

    def repl(m):
        if m.group(2) in names and m.group(3) == old:
            done.append(m.group(2))
            return f'{m.group(1)}version = "{new}"'
        return m.group(0)

    out = re.sub(r'(\[\[package\]\]\nname = "([^"]+)"\n)version = "([^"]+)"', repl, text)
    return out, done


def bump_pubspec_app(text, old, what):
    """version: X.Y.Z+N -> returns (text, new build number) for a caller-supplied new version."""
    m = re.search(r"^version:\s*" + re.escape(old) + r"\+(\d+)[ \t]*$", text, re.M)
    if not m:
        raise ReleaseError(f"{what}: expected 'version: {old}+N'")
    return int(m.group(1))


def set_pubspec_app(text, old, new, build, what):
    return _sub_once(r"^version:\s*" + re.escape(old) + r"\+\d+[ \t]*$", f"version: {new}+{build}", text, what)


def subject_of(message, new):
    """The commit subject: the message's first line, prefixed with vX.Y.Z when it has no version."""
    first = message.strip().splitlines()[0].strip()
    if not first:
        raise ReleaseError("empty commit message")
    if re.search(r"(^|[^\d.])v?" + re.escape(new) + r"(?![\d.]*\d)", first):
        return first
    return f"v{new}: {first}"


def title_of(message, new):
    """One-line changelog title: the subject without a leading 'vX.Y.Z:' and trailing period."""
    first = message.strip().splitlines()[0].strip()
    first = re.sub(r"^v?" + re.escape(new) + r"\s*[:\-]\s*", "", first)
    return first.rstrip(".").strip()


def move_changelog(text, new, date, app_build, title, allow_empty=False):
    """'## Unreleased' notes -> '## vX.Y.Z (date)' with the title and versions lines on top."""
    m = re.search(r"^## Unreleased[ \t]*\n(.*?)(?=^## |\Z)", text, re.S | re.M)
    if not m:
        raise ReleaseError("CHANGELOG.md has no '## Unreleased' section")
    notes = m.group(1).strip("\n")
    if not notes.strip() and not allow_empty:
        raise ReleaseError("CHANGELOG.md '## Unreleased' is empty (add notes, or pass --allow-empty-changelog)")
    if re.search(r"^## v" + re.escape(new) + r"\b", text, re.M):
        raise ReleaseError(f"CHANGELOG.md already has a v{new} section")
    sec = [f"## v{new} ({date})", f"itcmdr {new}: {title}.", f"- Versions {new} (app and admin app {new}+{app_build})."]
    if notes.strip():
        sec.append(notes)
    block = "## Unreleased\n\n" + "\n".join(sec) + "\n\n"
    return text[: m.start()] + block + text[m.end():]


def update_handover(text, old, new, title):
    needle = f"Latest tag: **v{old}**"
    if text.count(needle) != 1:
        raise ReleaseError(f"{HANDOVER}: expected exactly one '{needle}'")
    return text.replace(needle, f"Latest tag: **v{new}** ({title}). Previous: **v{old}**")


def update_plan(text, old, new, title, date):
    lines = text.split("\n")
    idx = [i for i, l in enumerate(lines) if l.startswith(f"| v{old} |")]
    if len(idx) != 1:
        raise ReleaseError(f"{PLAN}: expected exactly one row '| v{old} |'")
    if any(l.startswith(f"| v{new} |") for l in lines):
        raise ReleaseError(f"{PLAN}: already has a v{new} row")
    lines.insert(idx[0] + 1, f"| v{new} | {title.replace('|', '/')} | ✅ Released {date} |")
    return "\n".join(lines)


def today(tz=TZ):
    if ZoneInfo is None:
        raise ReleaseError("python zoneinfo is required (Python 3.9+)")
    return datetime.datetime.now(ZoneInfo(tz)).date().isoformat()


def parse_args(argv):
    p = argparse.ArgumentParser(description="Bump, changelog, commit, tag and push an itcmdr release.")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--major", dest="part", action="store_const", const="major", help="bump X (X+1.0.0)")
    g.add_argument("--minor", dest="part", action="store_const", const="minor", help="bump Y (default)")
    g.add_argument("--patch", dest="part", action="store_const", const="patch", help="bump Z")
    p.set_defaults(part="minor")
    p.add_argument("message", help="commit message (first line is the subject and changelog title)")
    p.add_argument("--dry-run", action="store_true", help="print what would happen, change nothing")
    p.add_argument("--no-push", action="store_true", help="commit and tag locally, do not push")
    p.add_argument("--remote", default="origin")
    p.add_argument("--allow-empty-changelog", action="store_true")
    return p.parse_args(argv)


# --- repo work -----------------------------------------------------------------------------------

def git(root, *args, check=True):
    r = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise ReleaseError(f"git {' '.join(args)} failed: {(r.stderr or r.stdout).strip()}")
    return r.stdout.strip()


def preflight(root, remote, tag):
    if git(root, "status", "--porcelain"):
        raise ReleaseError("the working tree is not clean (commit or stash first)")
    br = git(root, "rev-parse", "--abbrev-ref", "HEAD")
    if br != BRANCH:
        raise ReleaseError(f"releases are cut on {BRANCH}, not {br!r}")
    if git(root, "rev-parse", "-q", "--verify", f"refs/tags/{tag}", check=False):
        raise ReleaseError(f"tag {tag} already exists locally")
    remote_tag = git(root, "ls-remote", "--tags", remote, f"refs/tags/{tag}")
    if remote_tag:
        raise ReleaseError(f"tag {tag} already exists on {remote}")
    remote_main = git(root, "ls-remote", remote, f"refs/heads/{BRANCH}").split()
    if remote_main:
        sha = remote_main[0]
        have = subprocess.run(["git", "cat-file", "-e", f"{sha}^{{commit}}"], cwd=root, capture_output=True).returncode == 0
        if not have or subprocess.run(["git", "merge-base", "--is-ancestor", sha, "HEAD"], cwd=root).returncode != 0:
            raise ReleaseError(f"HEAD is behind {remote}/{BRANCH} ({sha[:12]}): pull first")


def check_dist_signatures(dist):
    """Fail closed when dist/<version> already holds a package without its pacman .sig."""
    if not dist.is_dir():
        return
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from check_pkg_sigs import check
    bad = check(dist)
    if bad:
        raise ReleaseError("unsigned or badly signed packages in " + str(dist) + ": " + "; ".join(bad))


def plan(root, part, message, allow_empty=False, date=None):
    """Compute every file change in memory. Returns (old, new, {path: new text}, info)."""
    rd = lambda p: (root / p).read_text(encoding="utf-8")
    _, old = cargo_package(rd("server/Cargo.toml"))
    new = bump(old, part)
    date = date or today()
    changes = {}
    names = set()
    for c in CRATES:
        p = f"{c}/Cargo.toml"
        t = rd(p)
        name, v = cargo_package(t)
        if v != old:
            raise ReleaseError(f"{p} is at {v}, expected {old} (versions out of step)")
        names.add(name)
        changes[p] = bump_cargo_toml(t, old, new, p)
    for p in LOCKS:
        t, done = bump_lock(rd(p), names, old, new)
        if not done:
            raise ReleaseError(f"{p}: no product package at {old}")
        stale = [n for n in bump_lock(t, names, old, new)[1]]
        if stale:
            raise ReleaseError(f"{p}: still at {old}: {stale}")
        changes[p] = t
    builds = set()
    for p in APP_PUBSPECS:
        builds.add(bump_pubspec_app(rd(p), old, p))
    if len(builds) != 1:
        raise ReleaseError(f"app and admin app build numbers differ: {sorted(builds)}")
    build = builds.pop() + 1
    for p in APP_PUBSPECS:
        changes[p] = set_pubspec_app(rd(p), old, new, build, p)
    changes[SITE_PUBSPEC] = _sub_once(r"^version:\s*" + re.escape(old) + r"[ \t]*$", f"version: {new}", rd(SITE_PUBSPEC), SITE_PUBSPEC)
    changes[AGENT_PS1] = _sub_once(r"^\$script:AgentVersion = '" + re.escape(old) + r"'", f"$script:AgentVersion = '{new}'", rd(AGENT_PS1), AGENT_PS1)
    changes[INSTALLER] = _sub_once(r'(#define MyAppVersion )"' + re.escape(old) + r'"', rf'\g<1>"{new}"', rd(INSTALLER), INSTALLER)
    title = title_of(message, new)
    if not title:
        raise ReleaseError("the commit message has no title")
    changes[CHANGELOG] = move_changelog(rd(CHANGELOG), new, date, build, title, allow_empty)
    changes[HANDOVER] = update_handover(rd(HANDOVER), old, new, title)
    changes[PLAN] = update_plan(rd(PLAN), old, new, title, date)
    return old, new, changes, {"build": build, "date": date, "title": title}


def main(argv=None, root=None):
    a = parse_args(sys.argv[1:] if argv is None else argv)
    root = Path(root or ROOT)
    try:
        _, old0 = cargo_package((root / "server/Cargo.toml").read_text(encoding="utf-8"))
        tag = f"v{bump(old0, a.part)}"
        preflight(root, a.remote, tag)
        check_dist_signatures(root / "dist" / tag[1:])
        old, new, changes, info = plan(root, a.part, a.message, a.allow_empty_changelog)
        subject = subject_of(a.message, new)
        body = "\n".join(a.message.strip().splitlines()[1:]).strip()
        print(f"itcmdr {old} -> {new} ({a.part}); app and admin app {new}+{info['build']}; {info['date']} ({TZ})")
        for p in sorted(changes):
            print(f"  update {p}")
        print(f"  run scripts/gen-help.py ({', '.join(GENERATED)})")
        print(f"  commit: {subject}")
        print(f"  annotated tag: {tag}")
        print(f"  push: {'skipped (--no-push)' if a.no_push else f'git push --follow-tags {a.remote} {BRANCH}'}")
        if a.dry_run:
            print("dry run: nothing changed")
            return 0
        try:
            for p, t in changes.items():
                (root / p).write_text(t, encoding="utf-8")
            gen = root / "scripts" / "gen-help.py"
            subprocess.run([sys.executable, str(gen)], cwd=root, check=True)
            subprocess.run([sys.executable, str(gen), "--check"], cwd=root, check=True)
            git(root, "add", "--", *sorted(changes), *GENERATED)
            msg = subject + ("\n\n" + body if body else "")
            git(root, "commit", "-q", "-m", msg)
        except Exception:
            # Nothing committed yet: put the tree back as it was (it was clean).
            git(root, "reset", "-q", "HEAD", check=False)
            git(root, "checkout", "-q", "--", ".", check=False)
            raise
        if git(root, "status", "--porcelain"):
            raise ReleaseError("files changed outside the release set (commit made, no tag); inspect `git status`")
        git(root, "tag", "-a", tag, "-m", f"itcmdr {new}: {info['title']}")
        print(f"committed {git(root, 'rev-parse', '--short', 'HEAD')} and tagged {tag}")
        if not a.no_push:
            git(root, "push", "--follow-tags", a.remote, BRANCH)
            print(f"pushed {BRANCH} and {tag} to {a.remote}")
        print(f"released version {new} (tag {tag}). Build every asset from {tag}: docs/RELEASE-LOCAL.md")
        return 0
    except (ReleaseError, subprocess.CalledProcessError, OSError) as e:
        print(f"release: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
