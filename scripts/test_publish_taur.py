#!/usr/bin/env python3
"""Tests for publish_taur.py: stub gh, curl, taurctl and vercmp; real git and makepkg; never the real
taurctl config (TAURCTL_CONFIG_DIR is a temp dir) and never the network."""
import hashlib, io, json, os, shutil, subprocess, sys, tarfile, tempfile, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import publish_taur as pt  # noqa: E402

VER = "1.4.0"
SLUG = "WeirdBrains/itcmdr"
GH = f"https://github.com/{SLUG}/releases/download/v{VER}"
SERVER = "https://taur.test"

STUB = r'''#!/usr/bin/env python3
import json, os, sys
state = os.environ["STUB_STATE"]
name = os.path.basename(sys.argv[0])
with open(os.path.join(state, "calls.log"), "a") as f:
    f.write(json.dumps([name] + sys.argv[1:]) + "\n")
s = json.load(open(os.path.join(state, "state.json")))
def save(): json.dump(s, open(os.path.join(state, "state.json"), "w"))
a = sys.argv[1:]
if name == "curl":
    url, dest = a[-1], a[a.index("-o") + 1]
    assert "-q" in a and a[a.index("--netrc-file") + 1] == os.devnull, "curl must be anonymous"
    src = s["urls"].get(url)
    if src is None:
        sys.stderr.write("curl: (22) The requested URL returned error: 404\n"); sys.exit(22)
    open(dest, "wb").write(open(src, "rb").read())
elif name == "gh":
    if a[:2] == ["repo", "view"]:
        seq = s["visibility"]
        v = seq.pop(0) if len(seq) > 1 else seq[0]
        save()
        if v is None:
            sys.stderr.write("gh: authentication required\n"); sys.exit(4)
        print(v)
    elif a[:2] == ["release", "view"]:
        if s["release"] is None:
            sys.stderr.write("release not found\n"); sys.exit(1)
        print(json.dumps(s["release"]))
    else:
        sys.exit(9)
elif name == "vercmp":
    k = lambda v: [int(x) for x in v.replace("-", ".").split(".")]
    x, y = k(a[0]), k(a[1]); print((x > y) - (x < y))
elif name == "taurctl":
    assert os.environ.get("TAURCTL_CONFIG_DIR", "").startswith(state), "real taurctl config"
    if "info" in a:
        print(json.dumps({"resultcount": 1 if s["taur"] else 0, "results": [s["taur"]] if s["taur"] else []}))
    elif "upload" in a:
        if s.get("upload_exit"):
            sys.stderr.write("not signed in\n"); sys.exit(s["upload_exit"])
        import tarfile
        arc = a[a.index("upload") + 1]
        with tarfile.open(arc) as t:
            for m in t.getmembers():
                p = os.path.join(state, "served-" + os.path.basename(m.name))
                data = t.extractfile(m).read()
                if s.get("corrupt_served"): data += b"#"
                open(p, "wb").write(data)
                s["urls"][f"{s['server']}/cgit/aur.git/plain/{os.path.basename(m.name)}?h=itcmdr-bin"] = p
        s["taur"] = {"PackageBase": "itcmdr-bin", "Name": "itcmdr-bin", "Version": s.get("served_version") or s["want_version"], "Organization": "WeirdBrains", "Maintainer": "m3talsmith"}
        save()
    else:
        sys.exit(9)
'''


def tgz(name, files):
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as t:
        d = tarfile.TarInfo(name); d.type = tarfile.DIRTYPE; d.mode = 0o755; t.addfile(d)
        for n, data in files.items():
            ti = tarfile.TarInfo(f"{name}/{n}"); ti.size = len(data); ti.mode = 0o755; t.addfile(ti, io.BytesIO(data))
    return buf.getvalue()


@unittest.skipUnless(shutil.which("makepkg") and shutil.which("git"), "needs makepkg and git")
class PublishTaur(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="test-publish-taur-"))
        self.state = self.tmp / "stub"; (self.state / "bin").mkdir(parents=True)
        (self.state / "taurctl-config").mkdir()
        for n in ("gh", "curl", "taurctl", "vercmp"):
            p = self.state / "bin" / n; p.write_text(STUB); p.chmod(0o755)
        self.env = {k: os.environ.get(k) for k in ("ITCMDR_GH", "ITCMDR_CURL", "ITCMDR_TAURCTL", "ITCMDR_VERCMP", "STUB_STATE", "TAURCTL_CONFIG_DIR")}
        for n in ("gh", "curl", "taurctl", "vercmp"):
            os.environ[f"ITCMDR_{n.upper()}"] = str(self.state / "bin" / n)
        os.environ["STUB_STATE"] = str(self.state)
        os.environ["TAURCTL_CONFIG_DIR"] = str(self.state / "taurctl-config")
        # A repo whose origin is GitHub-shaped; the tag check goes through a local bare remote.
        self.repo = self.tmp / "repo"; bare = self.tmp / "origin.git"
        g = lambda *a, cwd=self.repo: subprocess.run(["git", *a], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()
        subprocess.run(["git", "init", "-q", "--bare", str(bare)], check=True)
        self.repo.mkdir(); g("init", "-q", "-b", "main")
        g("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "release")
        g("-c", "user.name=t", "-c", "user.email=t@t", "tag", "-a", f"v{VER}", "-m", f"v{VER}")
        g("remote", "add", "origin", f"git@github.com:{SLUG}.git")
        g("config", "url." + str(bare) + ".insteadOf", f"git@github.com:{SLUG}.git")
        g("push", "-q", "origin", "main", f"v{VER}")
        self.commit = g("rev-parse", f"v{VER}^{{commit}}")
        self.git = g
        self.old_root = pt.ROOT; pt.ROOT = self.repo
        self.files = self.tmp / "files"; self.files.mkdir()
        self.urls = {}
        self.assets = {}
        for b in pt.BINS:
            self.add_tarball(b)
        self.write_sums()
        self.put("https://api.github.com/repos/" + SLUG, json.dumps({"full_name": SLUG, "private": False, "visibility": "public"}).encode())
        self.s = {"visibility": ["PUBLIC"], "taur": None, "server": SERVER, "want_version": f"{VER}-1",
                  "release": self.release()}
        self.logs = []

    def tearDown(self):
        pt.ROOT = self.old_root
        for k, v in self.env.items():
            if v is None: os.environ.pop(k, None)
            else: os.environ[k] = v
        shutil.rmtree(self.tmp)

    # fixtures
    def put(self, url, data):
        p = self.files / hashlib.sha1(url.encode()).hexdigest(); p.write_bytes(data); self.urls[url] = str(p)
        return data

    def add_tarball(self, b, *, ver=VER, commit=None, extra=None):
        top = f"{b}-{VER}-linux-x86_64"
        binary = b"\x7fELF..." + f"ITCMDR-BUILD v={ver} commit={commit or self.commit} help=93f60d64f75dfdc7 END".encode()
        files = {"README.txt": b"readme\n", b: binary, **(extra or {})}
        data = self.put(f"{GH}/{pt.tarball(VER, b)}", tgz(top, files))
        self.assets[pt.tarball(VER, b)] = len(data)

    def write_sums(self, override=None):
        lines = []
        for n in self.assets:
            h = hashlib.sha256(Path(self.urls[f"{GH}/{n}"]).read_bytes()).hexdigest()
            lines.append(f"{(override or {}).get(n, h)}  {n}")
        data = self.put(f"{GH}/SHA256SUMS", ("\n".join(lines) + "\n").encode())
        self.assets["SHA256SUMS"] = len(data)

    def release(self, **kw):
        r = {"tagName": f"v{VER}", "isDraft": False, "assets": [{"name": n, "size": s} for n, s in self.assets.items()]}
        r.update(kw); return r

    def go(self, **kw):
        self.s["urls"] = self.urls
        (self.state / "state.json").write_text(json.dumps(self.s))
        (self.state / "calls.log").write_text("")
        return pt.publish(VER, server=SERVER, log=self.logs.append, **kw)

    def calls(self, tool=None):
        out = [json.loads(l) for l in (self.state / "calls.log").read_text().splitlines()]
        return [c for c in out if tool is None or c[0] == tool]

    def uploads(self):
        return [c for c in self.calls("taurctl") if "upload" in c]

    def refuse(self, pattern, **kw):
        with self.assertRaisesRegex(pt.Refused, pattern):
            self.go(**kw)
        self.assertEqual(self.uploads(), [], "a refused publish must not upload")

    # tests
    def test_dry_run_checks_everything_and_uploads_nothing(self):
        self.assertEqual(self.go(dry_run=True), "dry-run")
        self.assertEqual(self.uploads(), [])
        self.assertTrue(any(c[1:3] == ["repo", "view"] for c in self.calls("gh")))
        self.assertTrue(any("info" in c for c in self.calls("taurctl")))
        self.assertIn("dry run: nothing was uploaded", self.logs)
        self.assertFalse(any(c[1:2] not in (["repo"], ["release"]) for c in self.calls("gh")), "gh only reads")

    def test_publishes_exact_recipe_and_verifies(self):
        self.assertEqual(self.go(), "published")
        (up,) = self.uploads()
        self.assertEqual(up[1:5], ["--server", SERVER, "pkg", "upload"])
        self.assertEqual(up[6:], ["--org", "WeirdBrains", "-y"])
        self.assertEqual(sum(1 for c in self.calls("gh") if c[1:3] == ["repo", "view"]), 2, "public checked twice")
        served = (self.state / "served-.SRCINFO").read_text()
        self.assertIn(f"pkgver = {VER}", served)
        self.assertIn(f"source = {GH}/itcmdr-cli-{VER}-linux-x86_64.tar.gz", served)
        self.assertNotIn("SKIP", (self.state / "served-PKGBUILD").read_text())

    def test_archive_holds_only_pkgbuild_and_srcinfo(self):
        out = self.tmp / "out"
        self.go(dry_run=True, out=out)
        with tarfile.open(out / f"itcmdr-bin-{VER}-1.src.tar.gz") as t:
            self.assertEqual(sorted(m.name for m in t.getmembers()), ["itcmdr-bin/.SRCINFO", "itcmdr-bin/PKGBUILD"])
            self.assertTrue(all(m.uid == 0 and m.mtime == 0 and m.isfile() for m in t.getmembers()))

    def test_not_public_never_uploads(self):
        for vis in ("PRIVATE", "INTERNAL", "", None):
            with self.subTest(vis=vis):
                self.s["visibility"] = [vis]
                self.refuse("not confirmed PUBLIC")

    def test_anonymous_api_must_say_public(self):
        for meta in ({"full_name": SLUG, "private": True, "visibility": "private"},
                     {"full_name": SLUG, "visibility": "public"}, {"full_name": "Other/x", "private": False, "visibility": "public"}):
            with self.subTest(meta=meta):
                self.put("https://api.github.com/repos/" + SLUG, json.dumps(meta).encode())
                self.refuse("does not report")
        del self.urls["https://api.github.com/repos/" + SLUG]
        self.refuse("not visible anonymously")

    def test_visibility_rechecked_right_before_upload(self):
        self.s["visibility"] = ["PUBLIC", "PRIVATE"]
        self.refuse("not confirmed PUBLIC")
        self.assertEqual(sum(1 for c in self.calls("gh") if c[1:3] == ["repo", "view"]), 2)

    def test_release_must_be_published_and_complete(self):
        self.s["release"] = None; self.refuse("release not found")
        self.s["release"] = self.release(isDraft=True); self.refuse("not a published GitHub release")
        r = self.release(); r["assets"] = [a for a in r["assets"] if a["name"] != "SHA256SUMS"]
        self.s["release"] = r; self.refuse("lacks SHA256SUMS")

    def test_package_asset_without_signature_refused(self):
        for sig in (None, 0):
            with self.subTest(sig=sig):
                r = self.release(); r["assets"].append({"name": f"itcmdr-bin-{VER}-1-x86_64.pkg.tar.zst", "size": 10})
                if sig is not None:
                    r["assets"].append({"name": f"itcmdr-bin-{VER}-1-x86_64.pkg.tar.zst.sig", "size": sig})
                self.s["release"] = r
                self.refuse(r"no non-empty .*\.sig")
        r = self.release(); r["assets"] += [{"name": "x.pkg.zst", "size": 3}, {"name": "x.pkg.zst.sig", "size": 119}]
        self.s["release"] = r
        self.assertEqual(self.go(dry_run=True), "dry-run")

    def test_checksum_mismatch_refused(self):
        self.write_sums({pt.tarball(VER, "itcmdr-cli"): "0" * 64})
        self.s["release"] = self.release()
        self.refuse("does not match SHA256SUMS")

    def test_wrong_stamp_refused(self):
        for kw in ({"commit": "a" * 40}, {"ver": "1.3.0"}):
            with self.subTest(**kw):
                self.add_tarball("itcmdr-admin", **kw); self.write_sums(); self.s["release"] = self.release()
                self.refuse("is not stamped")

    def test_unexpected_tarball_member_refused(self):
        self.add_tarball("itcmdr-cli", extra={"notes.txt": b"x"}); self.write_sums(); self.s["release"] = self.release()
        self.refuse("expected")

    def test_tag_must_be_annotated_and_on_origin(self):
        self.git("tag", "-d", f"v{VER}"); self.git("tag", f"v{VER}")
        self.refuse("not an annotated tag")
        self.git("tag", "-d", f"v{VER}")
        self.git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "later")
        self.git("-c", "user.name=t", "-c", "user.email=t@t", "tag", "-a", f"v{VER}", "-m", "moved")
        self.refuse("origin's v1.4.0 is not at")

    def test_same_version_same_recipe_is_unchanged(self):
        self.assertEqual(self.go(), "published")
        self.s = json.loads((self.state / "state.json").read_text()); self.urls = self.s["urls"]
        self.assertEqual(self.go(), "unchanged")
        self.assertEqual(self.uploads(), [])

    def test_same_version_different_recipe_refused(self):
        self.s["corrupt_served"] = True
        with self.assertRaisesRegex(pt.Refused, "serves a different"):
            self.go()
        self.s = json.loads((self.state / "state.json").read_text()); self.urls = self.s["urls"]
        self.s["corrupt_served"] = False
        self.refuse("different recipe; not replacing")

    def test_newer_or_foreign_taur_package_refused(self):
        self.s["taur"] = {"PackageBase": "itcmdr-bin", "Version": "1.5.0-1", "Organization": "WeirdBrains"}
        self.refuse("not older than")
        self.s["taur"] = {"PackageBase": "itcmdr-bin", "Version": "1.0.0-1", "Organization": "someone"}
        self.refuse("belongs to someone")

    def test_older_taur_package_is_updated(self):
        self.s["taur"] = {"PackageBase": "itcmdr-bin", "Version": "1.3.0-1", "Organization": "WeirdBrains"}
        self.assertEqual(self.go(), "published")

    def test_upload_failure_and_bad_verification_reported(self):
        self.s["upload_exit"] = 3
        with self.assertRaisesRegex(pt.Refused, "not signed in"):
            self.go()
        self.s["upload_exit"] = 0; self.s["served_version"] = "1.3.0-1"
        with self.assertRaisesRegex(pt.Refused, "after the upload TAUR reports 1.3.0-1"):
            self.go()

    def test_out_never_overwrites_different_content(self):
        out = self.tmp / "out"; out.mkdir()
        dest = out / f"itcmdr-bin-{VER}-1.src.tar.gz"; dest.write_bytes(b"other")
        self.refuse("not overwriting", out=out)
        self.assertEqual(dest.read_bytes(), b"other")
        dest.unlink(); self.go(dry_run=True, out=out); first = dest.read_bytes()
        self.go(dry_run=True, out=out); self.assertEqual(dest.read_bytes(), first, "deterministic")

    def test_bad_inputs_refused(self):
        with self.assertRaisesRegex(pt.Refused, "X.Y.Z"):
            pt.publish("1.4", log=self.logs.append)
        with self.assertRaisesRegex(pt.Refused, "https"):
            pt.publish(VER, server="http://taur.test", log=self.logs.append)
        self.git("remote", "set-url", "origin", "https://gitlab.com/x/y.git")
        self.refuse("not a GitHub repository")

    def test_main_exit_code(self):
        self.s["visibility"] = ["PRIVATE"]
        self.s["urls"] = self.urls; (self.state / "state.json").write_text(json.dumps(self.s))
        self.assertEqual(pt.main([VER, "--server", SERVER]), 1)


if __name__ == "__main__":
    unittest.main()
