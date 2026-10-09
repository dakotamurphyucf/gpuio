#!/usr/bin/env python3
"""Offline reconstruction admission tests; never touch the repository vendor tree."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest


class Reconstruction(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="gpuio-vendor-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "scripts").mkdir()
        shutil.copyfile(Path(__file__).with_name("vendor_bonsai.py"),
                        self.root / "scripts/vendor_bonsai.py")
        (self.root / "third_party/patches").mkdir(parents=True)
        self.archives = self.root / "archives"
        self.archives.mkdir()
        self.manifest = {"native_bonsai": {}}
        for name in ("bonsai", "virtual_dom"):
            commit = "1" * 40
            archive = self.archives / f"{name}.tar.gz"
            with tarfile.open(archive, "w:gz") as bundle:
                contents = b"upstream\n"
                info = tarfile.TarInfo(f"{name}-{commit}/README")
                info.size = len(contents)
                info.mode = 0o644
                bundle.addfile(info, io.BytesIO(contents))
            patch = self.root / f"third_party/patches/{name}.patch"
            patch.write_text("--- a/README\n+++ b/README\n@@ -1 +1 @@\n-upstream\n+reviewed\n")
            self.manifest["native_bonsai"][name] = {
                "commit": commit,
                "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                "patch_sha256": hashlib.sha256(patch.read_bytes()).hexdigest(),
            }
        self.manifest_path = self.root / "third_party/sources.json"
        self.manifest_path.write_text(json.dumps(self.manifest))
        self.original_manifest = self.manifest_path.read_bytes()
        self.output = self.root / "output/nested/vendor"
        # No curl is available to the child: these cases must stay offline.
        tools = self.root / "tools"
        tools.mkdir()
        patch = shutil.which("gpatch") or shutil.which("patch")
        self.assertIsNotNone(patch, "reconstruction requires patch")
        (tools / "patch").symlink_to(patch)
        self.env = {**os.environ, "PATH": str(tools)}

    def reconstruct(self, *arguments):
        return subprocess.run(
            [sys.executable, str(self.root / "scripts/vendor_bonsai.py"),
             "--archive-dir", str(self.archives), "--output", str(self.output),
             *arguments], env=self.env, capture_output=True, text=True, timeout=20)

    def test_full_offline_reconstruction_preserves_live_tree_and_manifest(self):
        live = self.root / "vendor"
        live.mkdir()
        (live / "sentinel").write_text("keep")
        result = self.reconstruct()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sorted(p.name for p in self.output.iterdir()), ["bonsai", "virtual_dom"])
        for name in self.manifest["native_bonsai"]:
            self.assertEqual((self.output / name / "README").read_text(), "reviewed\n")
        self.assertEqual((live / "sentinel").read_text(), "keep")
        self.assertEqual(self.manifest_path.read_bytes(), self.original_manifest)

    def test_explicit_subset_never_requires_an_unselected_archive(self):
        (self.archives / "virtual_dom.tar.gz").unlink()
        result = self.reconstruct("--package", "bonsai")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([p.name for p in self.output.iterdir()], ["bonsai"])
        self.assertEqual(self.manifest_path.read_bytes(), self.original_manifest)

    def test_missing_archive_never_downloads_or_publishes_partial_output(self):
        (self.archives / "virtual_dom.tar.gz").unlink()
        result = self.reconstruct()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Missing local archive", result.stderr)
        self.assertFalse(self.output.exists())

    def test_archive_and_patch_corruption_fail_before_output_creation(self):
        for path, message in [
            (self.archives / "virtual_dom.tar.gz", "Archive checksum mismatch"),
            (self.root / "third_party/patches/virtual_dom.patch", "Patch checksum mismatch"),
        ]:
            with self.subTest(path=path):
                original = path.read_bytes()
                try:
                    path.write_bytes(original + b"corrupted")
                    result = self.reconstruct()
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(message, result.stderr)
                    self.assertFalse(self.output.exists())
                    self.assertEqual(self.manifest_path.read_bytes(), self.original_manifest)
                finally:
                    path.write_bytes(original)

    def test_existing_output_and_dangling_output_link_are_not_overwritten(self):
        self.output.mkdir(parents=True)
        (self.output / "sentinel").write_text("keep")
        result = self.reconstruct()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Output already exists", result.stderr)
        self.assertEqual((self.output / "sentinel").read_text(), "keep")
        (self.output / "sentinel").unlink()
        self.output.rmdir()
        target = self.root / "absent"
        self.output.symlink_to(target)
        result = self.reconstruct()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Output already exists", result.stderr)
        self.assertTrue(self.output.is_symlink())
        self.assertFalse(target.exists())


if __name__ == "__main__":
    unittest.main()
