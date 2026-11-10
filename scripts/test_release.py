#!/usr/bin/env python3
"""Tests for scripts/release.py. Never touches this repo or its remote: the end-to-end cases run in a
throwaway git repo with a throwaway bare remote under the system temp dir.

    python3 scripts/test_release.py
"""
import contextlib, io, subprocess, sys, tempfile, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import release as r

CHANGELOG = """# Changelog

## Unreleased
- Added a thing.
- Fixed a thing.

## v1.3.8 (2026-11-06)
itcmdr 1.3.8: old.
- Versions 1.3.8 (app and admin app 1.3.8+50).
"""


class Bump(unittest.TestCase):
    def test_arithmetic(self):
        self.assertEqual(r.bump("1.3.8", "patch"), "1.3.9")
        self.assertEqual(r.bump("1.3.8", "minor"), "1.4.0")
        self.assertEqual(r.bump("1.3.8", "major"), "2.0.0")
        self.assertEqual(r.bump("0.9.9", "minor"), "0.10.0")
        with self.assertRaises(r.ReleaseError):
            r.bump("1.3", "patch")

    def test_flags(self):
        self.assertEqual(r.parse_args(["m"]).part, "minor")
        self.assertEqual(r.parse_args(["--patch", "m"]).part, "patch")
        self.assertEqual(r.parse_args(["--major", "m"]).part, "major")
        for bad in (["--major", "--patch", "m"], ["--minor", "--patch", "m"], []):
            with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
                r.parse_args(bad)

    def test_subject_and_title(self):
        self.assertEqual(r.subject_of("plugin tags", "1.4.0"), "v1.4.0: plugin tags")
        self.assertEqual(r.subject_of("v1.4.0: plugin tags", "1.4.0"), "v1.4.0: plugin tags")
        self.assertEqual(r.subject_of("fix 1.4.0 regression\n\nbody", "1.4.0"), "fix 1.4.0 regression")
        self.assertEqual(r.subject_of("from 1.4.01", "1.4.0"), "v1.4.0: from 1.4.01")
        self.assertEqual(r.title_of("v1.4.0: plugin tags.", "1.4.0"), "plugin tags")


class Changelog(unittest.TestCase):
    def test_heading_shape(self):
        out = r.move_changelog(CHANGELOG, "1.4.0", "2026-11-10", 51, "plugin tags")
        lines = out.splitlines()
        self.assertEqual(lines[:9], [
            "# Changelog", "", "## Unreleased", "", "## v1.4.0 (2026-11-10)", "itcmdr 1.4.0: plugin tags.",
            "- Versions 1.4.0 (app and admin app 1.4.0+51).", "- Added a thing.", "- Fixed a thing."])
        self.assertEqual(lines[9:11], ["", "## v1.3.8 (2026-11-06)"])
        self.assertEqual(out.count("## Unreleased"), 1)

    def test_refuses_empty_or_duplicate(self):
        empty = CHANGELOG.replace("- Added a thing.\n- Fixed a thing.\n", "")
        with self.assertRaises(r.ReleaseError):
            r.move_changelog(empty, "1.4.0", "2026-11-10", 51, "t")
        self.assertIn("## v1.4.0 (2026-11-10)", r.move_changelog(empty, "1.4.0", "2026-11-10", 51, "t", allow_empty=True))
        with self.assertRaises(r.ReleaseError):
            r.move_changelog(CHANGELOG, "1.3.8", "2026-11-10", 51, "t")

    def test_lock_only_named_packages(self):
        lock = ('[[package]]\nname = "itcmdr-server"\nversion = "1.3.8"\n\n'
                '[[package]]\nname = "itcmdr-common"\nversion = "1.3.8"\n\n'
                '[[package]]\nname = "serde"\nversion = "1.3.8"\nsource = "registry"\n')
        out, done = r.bump_lock(lock, {"itcmdr-server"}, "1.3.8", "1.4.0")
        self.assertEqual(done, ["itcmdr-server"])
        self.assertIn('name = "itcmdr-server"\nversion = "1.4.0"', out)
        self.assertIn('name = "itcmdr-common"\nversion = "1.3.8"', out)
        self.assertIn('name = "serde"\nversion = "1.3.8"', out)


def sh(cwd, *a):
    return subprocess.run(a, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


def fake_repo(tmp: Path):
    """A minimal tree with every file release.py edits, committed on main, with a bare origin."""
    root = tmp / "repo"
    names = {c: f"itcmdr-{c.replace('/', '-')}" for c in r.CRATES}
    for c, n in names.items():
        (root / c).mkdir(parents=True, exist_ok=True)
        (root / c / "Cargo.toml").write_text(f'[package]\nname = "{n}"\nversion = "1.3.8"\n\n[dependencies]\nx = {{ version = "1.3.8" }}\n')
    pkgs = lambda ns: "".join(f'[[package]]\nname = "{n}"\nversion = "1.3.8"\n\n' for n in ns)
    (root / "Cargo.lock").write_text(pkgs(list(names.values()) + ["itcmdr-common"]))
    (root / "app/rust/Cargo.lock").write_text(pkgs([names["app/rust"], names["keys"]]))
    (root / "admin-app/rust/Cargo.lock").write_text(pkgs([names["admin-app/rust"], names["keys"]]))
    for p in r.APP_PUBSPECS:
        (root / p).write_text("name: x\nversion: 1.3.8+50\n")
    (root / "site_jaspr").mkdir()
    (root / r.SITE_PUBSPEC).write_text("name: s\nversion: 1.3.8\n")
    (root / "agent").mkdir()
    (root / r.AGENT_PS1).write_text("$script:AgentVersion = '1.3.8'\n")
    (root / "installer").mkdir()
    (root / r.INSTALLER).write_text('#ifndef MyAppVersion\n  #define MyAppVersion "1.3.8"\n#endif\n')
    (root / r.CHANGELOG).write_text(CHANGELOG)
    (root / "docs").mkdir()
    (root / r.HANDOVER).write_text("- Branch: `main`. Latest tag: **v1.3.8** (old). Previous: **v1.3.7**.\n")
    (root / r.PLAN).write_text("| Tag | What | Status |\n|---|---|---|\n| v1.3.8 | old | ✅ Released 2026-11-06 |\n")
    (root / "scripts").mkdir()
    gen = ("import sys, pathlib\nroot = pathlib.Path(__file__).resolve().parent.parent\n"
           "body = 'help ' + (root / 'docs/HANDOVER.md').read_text()\n"
           "for a in ['app', 'admin-app']:\n    f = root / a / 'lib/help_topics.g.dart'\n"
           "    if '--check' in sys.argv:\n        sys.exit(0 if f.read_text() == body else 1)\n"
           "    f.parent.mkdir(parents=True, exist_ok=True); f.write_text(body)\n")
    (root / "scripts/gen-help.py").write_text(gen)
    subprocess.run([sys.executable, str(root / "scripts/gen-help.py")], check=True)
    sh(tmp, "git", "init", "-q", "--bare", "-b", "main", "origin.git")
    sh(root, "git", "init", "-q", "-b", "main")
    sh(root, "git", "config", "user.email", "t@example.invalid")
    sh(root, "git", "config", "user.name", "test")
    sh(root, "git", "config", "commit.gpgsign", "false")
    sh(root, "git", "config", "tag.gpgsign", "false")
    sh(root, "git", "add", "-A")
    sh(root, "git", "commit", "-q", "-m", "init")
    sh(root, "git", "remote", "add", "origin", str(tmp / "origin.git"))
    sh(root, "git", "push", "-q", "origin", "main")
    return root


class EndToEnd(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.tmp = Path(self.t.name)
        self.root = fake_repo(self.tmp)

    def tearDown(self):
        self.t.cleanup()

    def run_release(self, *args):
        o, e = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(o), contextlib.redirect_stderr(e):
            code = r.main(list(args), root=self.root)
        return code, o.getvalue(), e.getvalue()

    def test_dry_run_changes_nothing(self):
        head = sh(self.root, "git", "rev-parse", "HEAD")
        code, out, _ = self.run_release("--dry-run", "--patch", "small fix")
        self.assertEqual(code, 0)
        self.assertIn("1.3.8 -> 1.3.9", out)
        self.assertEqual(sh(self.root, "git", "status", "--porcelain"), "")
        self.assertEqual(sh(self.root, "git", "rev-parse", "HEAD"), head)
        self.assertEqual(sh(self.root, "git", "tag"), "")

    def test_no_push_commits_and_tags_locally(self):
        code, out, err = self.run_release("--no-push", "plugin tags")
        self.assertEqual(code, 0, err)
        self.assertEqual(sh(self.root, "git", "log", "-1", "--format=%s"), "v1.4.0: plugin tags")
        self.assertEqual(sh(self.root, "git", "cat-file", "-t", "v1.4.0"), "tag")
        self.assertEqual(sh(self.root, "git", "status", "--porcelain"), "")
        self.assertEqual(sh(self.tmp / "origin.git", "git", "tag"), "")
        self.assertIn('version = "1.4.0"', (self.root / "server/Cargo.toml").read_text())
        self.assertIn('x = { version = "1.3.8" }', (self.root / "server/Cargo.toml").read_text())
        self.assertIn("version: 1.4.0+51\n", (self.root / "app/pubspec.yaml").read_text())
        self.assertIn("version: 1.4.0\n", (self.root / r.SITE_PUBSPEC).read_text())
        self.assertIn("'1.4.0'", (self.root / r.AGENT_PS1).read_text())
        self.assertIn('"1.4.0"', (self.root / r.INSTALLER).read_text())
        lock = (self.root / "Cargo.lock").read_text()
        self.assertIn('name = "itcmdr-common"\nversion = "1.3.8"', lock)
        self.assertEqual(lock.count('version = "1.4.0"'), len(r.CRATES))
        self.assertIn("Latest tag: **v1.4.0** (plugin tags). Previous: **v1.3.8** (old)", (self.root / r.HANDOVER).read_text())
        self.assertIn("| v1.4.0 | plugin tags | ✅ Released", (self.root / r.PLAN).read_text())
        self.assertTrue((self.root / "app/lib/help_topics.g.dart").read_text().startswith("help - Branch"))
        self.assertIn("v1.4.0", (self.root / "app/lib/help_topics.g.dart").read_text())

    def test_push_sends_commit_and_tag_to_the_throwaway_remote(self):
        code, _, err = self.run_release("--major", "big one")
        self.assertEqual(code, 0, err)
        self.assertEqual(sh(self.tmp / "origin.git", "git", "tag"), "v2.0.0")
        self.assertEqual(sh(self.tmp / "origin.git", "git", "rev-parse", "main"), sh(self.root, "git", "rev-parse", "HEAD"))

    def test_refuses_dirty_tree_other_branch_and_existing_tag(self):
        (self.root / "stray.txt").write_text("x")
        self.assertEqual(self.run_release("--no-push", "m")[0], 1)
        (self.root / "stray.txt").unlink()
        sh(self.root, "git", "checkout", "-q", "-b", "feature")
        self.assertEqual(self.run_release("--no-push", "m")[0], 1)
        sh(self.root, "git", "checkout", "-q", "main")
        sh(self.root, "git", "tag", "v1.4.0")
        code, _, err = self.run_release("--no-push", "m")
        self.assertEqual(code, 1)
        self.assertIn("already exists locally", err)

    def test_refuses_unsigned_package_in_dist(self):
        d = self.root / "dist" / "1.4.0"
        d.mkdir(parents=True)
        (d / "itcmdr-cli-1.4.0-1-x86_64.pkg.tar.zst").write_bytes(b"x")
        sh(self.root, "git", "config", "core.excludesFile", str(self.tmp / "ignore"))
        (self.tmp / "ignore").write_text("dist/\n")
        code, _, err = self.run_release("--dry-run", "m")
        self.assertEqual(code, 1)
        self.assertIn("missing signature", err)

    def test_failure_before_commit_restores_the_tree(self):
        (self.root / "scripts/gen-help.py").write_text("import sys; sys.exit(3)\n")
        sh(self.root, "git", "commit", "-qam", "break gen-help")
        sh(self.root, "git", "push", "-q", "origin", "main")
        head = sh(self.root, "git", "rev-parse", "HEAD")
        self.assertEqual(self.run_release("--no-push", "m")[0], 1)
        self.assertEqual(sh(self.root, "git", "status", "--porcelain"), "")
        self.assertEqual(sh(self.root, "git", "rev-parse", "HEAD"), head)
        self.assertEqual(sh(self.root, "git", "tag"), "")


if __name__ == "__main__":
    unittest.main(verbosity=2)
