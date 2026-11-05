#!/usr/bin/env python3
"""Tests for the release guard (scripts/check_stamps.py). Run: python3 scripts/test_check_stamps.py"""
import io, sys, tarfile, tempfile, unittest, zipfile
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_stamps as cs

C = "a" * 40
def stamp(c=C, h=None): return f"ITCMDR-BUILD v=1.3.1 commit={c} help={h or cs.help_hash()} END".encode()

def tgz(p, data):
    with tarfile.open(p, "w:gz") as t:
        i = tarfile.TarInfo("bin/x"); i.size = len(data); t.addfile(i, io.BytesIO(data))

def zp(p, data):
    with zipfile.ZipFile(p, "w") as z: z.writestr("a.app/lib", data)

class T(unittest.TestCase):
    def run_check(self, files, side=None):
        with tempfile.TemporaryDirectory() as d:
            d = Path(d); dist = d / "dist"; dist.mkdir()
            for name, fn, data in files: fn(dist / name, data)
            sf = None
            if side is not None: sf = d / "stamps.txt"; sf.write_text(side)
            return cs.check(dist, C, cs.help_hash(), sf)

    def test_all_match(self):
        self.assertEqual(self.run_check([("a.tar.gz", tgz, b"..." + stamp() + b"..."), ("b.zip", zp, stamp()),
                                         ("c.apk", zp, stamp())]), [])

    def test_other_commit(self):
        bad = self.run_check([("a.tar.gz", tgz, stamp(c="b" * 40))])
        self.assertTrue(any("built from" in b for b in bad))

    def test_stale_help(self):
        bad = self.run_check([("a.zip", zp, stamp(h="0" * 16))])
        self.assertTrue(any("help" in b for b in bad))

    def test_no_stamp(self):
        self.assertTrue(self.run_check([("a.tar.gz", tgz, b"nothing")]))

    def test_exe_uses_vm_stamps(self):
        w = lambda p, data: p.write_bytes(data)
        self.assertTrue(self.run_check([("x-Setup.exe", w, b"lzma")]))
        self.assertEqual(self.run_check([("x-Setup.exe", w, b"lzma")], f"x-Setup.exe: {stamp().decode()}\n"), [])
        self.assertTrue(self.run_check([("x-Setup.exe", w, b"lzma")], f"x-Setup.exe: {stamp(c='c'*40).decode()}\n"))

    def test_hash_matches_rust_format(self):
        self.assertRegex(cs.help_hash(), r"^[0-9a-f]{16}$")

if __name__ == "__main__":
    unittest.main()
