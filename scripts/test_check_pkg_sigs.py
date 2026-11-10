#!/usr/bin/env python3
"""Tests for scripts/check_pkg_sigs.py (pairing rule; gpg is stubbed, nothing is signed here).

    python3 scripts/test_check_pkg_sigs.py
"""
import os, sys, tempfile, unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_pkg_sigs as c


def stub_gpg(d: Path, status: str, code: int) -> str:
    """A fake gpg that prints the given --status-fd lines (no keys, no real signatures)."""
    p = d / "fake-gpg"
    p.write_text(f"#!/bin/sh\nprintf '%s\\n' {' '.join(repr(l) for l in status.splitlines())}\nexit {code}\n")
    p.chmod(0o755)
    return str(p)


class Pairing(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.d = Path(self.t.name) / "dist"
        self.d.mkdir()

    def tearDown(self):
        self.t.cleanup()

    def put(self, name, data=b"x"):
        (self.d / name).write_bytes(data)

    def test_package_without_sig_is_rejected(self):
        self.put("itcmdr-cli-1.4.0-1-x86_64.pkg.tar.zst")
        self.put("other-1.0-1-any.pkg.zst")
        bad = c.check(self.d, verify=False)
        self.assertEqual(len(bad), 2)
        self.assertTrue(all("missing signature" in b for b in bad))

    def test_matching_sig_passes_the_pair_check(self):
        self.put("itcmdr-cli-1.4.0-1-x86_64.pkg.tar.zst")
        self.put("itcmdr-cli-1.4.0-1-x86_64.pkg.tar.zst.sig", b"\x89sig")
        self.put("other-1.0-1-any.pkg.zst")
        self.put("other-1.0-1-any.pkg.zst.sig", b"\x89sig")
        self.put("itcmdr-cli-1.4.0-linux-x86_64.tar.gz")  # not a package: no .sig needed
        self.assertEqual(c.check(self.d, verify=False), [])

    def test_empty_or_misnamed_sig_is_rejected(self):
        self.put("a-1-1-any.pkg.tar.zst")
        self.put("a-1-1-any.pkg.tar.zst.sig", b"")
        self.put("b-1-1-any.pkg.tar.zst")
        self.put("b-1-1-any.pkg.sig")       # not named after the package file
        self.put("b-1-1-any.pkg.tar.sig")
        bad = c.check(self.d, verify=False)
        self.assertEqual(sorted(b.split(":")[0] for b in bad), ["a-1-1-any.pkg.tar.zst", "b-1-1-any.pkg.tar.zst"])
        self.assertIn("empty signature", " ".join(bad))

    def test_no_packages_is_fine(self):
        self.put("SHA256SUMS")
        self.assertEqual(c.check(self.d), [])

    def test_gpg_bad_signature_fails(self):
        self.put("a-1-1-any.pkg.tar.zst")
        self.put("a-1-1-any.pkg.tar.zst.sig", b"s")
        g = stub_gpg(Path(self.t.name), "[GNUPG:] BADSIG 0123 someone", 1)
        bad = c.check(self.d, gpg=g)
        self.assertEqual(len(bad), 1)
        self.assertIn("does not verify", bad[0])

    def test_gpg_good_signature_passes(self):
        self.put("a-1-1-any.pkg.tar.zst")
        self.put("a-1-1-any.pkg.tar.zst.sig", b"s")
        g = stub_gpg(Path(self.t.name), "[GNUPG:] GOODSIG 0123 someone\n[GNUPG:] VALIDSIG ABCD", 0)
        self.assertEqual(c.check(self.d, gpg=g), [])

    def test_no_public_key_still_needs_only_the_pair(self):
        self.put("a-1-1-any.pkg.tar.zst")
        self.put("a-1-1-any.pkg.tar.zst.sig", b"s")
        g = stub_gpg(Path(self.t.name), "[GNUPG:] ERRSIG 0123 1 10 00 1 9\n[GNUPG:] NO_PUBKEY 0123", 2)
        self.assertEqual(c.check(self.d, gpg=g), [])

    def test_unreadable_signature_fails(self):
        self.put("a-1-1-any.pkg.tar.zst")
        self.put("a-1-1-any.pkg.tar.zst.sig", b"not a signature")
        g = stub_gpg(Path(self.t.name), "[GNUPG:] NODATA 3", 2)
        self.assertEqual(len(c.check(self.d, gpg=g)), 1)

    def test_main_exit_codes(self):
        self.put("a-1-1-any.pkg.tar.zst")
        self.assertEqual(c.main(["x", str(self.d)]), 1)
        self.put("a-1-1-any.pkg.tar.zst.sig", b"s")
        old = os.environ.get("PATH", "")
        os.environ["PATH"] = ""  # no gpg: pairing only
        try:
            self.assertEqual(c.main(["x", str(self.d)]), 0)
        finally:
            os.environ["PATH"] = old


if __name__ == "__main__":
    unittest.main(verbosity=2)
