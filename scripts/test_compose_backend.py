#!/usr/bin/env python3
"""Portable tests of static backend composition; no compiler or display needed."""
import contextlib
import io
import json
import re
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest.mock import patch

from compose_backend import generate

ROOT = Path(__file__).resolve().parents[1]


class CompositionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        (self.root / "dune-project").write_text("(lang dune 3.17)\n")
        for name in ("shared", "reader"):
            package = self.root / name
            package.mkdir()
            (package / "Cargo.toml").write_text(f'[package]\nname = "{name}"\n')
        self.config = {"library": "app_native", "gpuio": str(ROOT), "components": []}
        self.manifest = self.root / "native.json"
        self.output = self.root / "backend"

    def generate(self):
        self.manifest.write_text(json.dumps(self.config))
        with contextlib.redirect_stdout(io.StringIO()):
            generate(self.manifest, self.output)
        return {name: (self.output / name).read_text() for name in
                ("Cargo.toml", "registration.rs", "dune", "backend.ml")}

    def test_profile_only_uses_combined_catalog_and_sdk_patch(self):
        self.config["document_profiles"] = [{"path": "reader", "factory": "reader::factory"}]
        files = self.generate()
        cargo = tomllib.loads(files["Cargo.toml"])
        self.assertEqual(cargo["dependencies"]["document_profile_0"]["path"], "../reader")
        self.assertEqual(cargo["patch"]["crates-io"]["gpuio-document-sdk"]["path"], str(ROOT / "rust/document-sdk"))
        self.assertIn("registrations::install([], [document_profile_0::reader::factory()])", files["registration.rs"])
        self.assertIn('(source_tree "../reader")', files["dune"])
        self.assertEqual(self.generate(), files)  # deterministic repeated generation

    def test_shared_package_has_one_cargo_identity_and_both_factories(self):
        self.config["components"] = [{"path": "shared", "factory": "component"}]
        self.config["document_profiles"] = [{"path": "shared", "factory": "profile"}, {"path": "reader", "factory": "profile"}]
        files = self.generate()
        cargo = tomllib.loads(files["Cargo.toml"])
        shared = [d for d in cargo["dependencies"].values() if d.get("package") == "shared"]
        self.assertEqual(len(shared), 1)
        self.assertIn("registrations::install([component_0::component()], [component_0::profile(), document_profile_1::profile()])", files["registration.rs"])
        self.assertEqual(files["dune"].count('(source_tree "../shared")'), 1)

    def test_legacy_manifest_stays_on_component_api(self):
        self.config["components"] = [{"path": "shared", "factory": "component"}]
        files = self.generate()
        self.assertIn("extensions::install([component_0::component()])", files["registration.rs"])
        self.assertNotIn("gpuio-document-sdk", files["Cargo.toml"])
        self.config["document_profiles"] = []
        self.assertEqual(files, self.generate())

    def test_existing_application_outputs_are_unchanged_apart_from_dune_formatting(self):
        for example in ("extension_consumer", "signal_studio"):
            base = ROOT / "examples" / example
            captured = {}
            def record(path, text):
                captured[path] = text
            exists = Path.exists
            def generated_absent(path):
                return False if path.parent == base / "backend" else exists(path)
            with patch.object(Path, "write_text", record), patch.object(Path, "exists", generated_absent), contextlib.redirect_stdout(io.StringIO()):
                generate(base / "native.json", base / "backend")
            self.assertEqual(len(captured), 4)
            for path, text in captured.items():
                if path.name == "dune":
                    # dune fmt expands lists/newlines without changing tokens.
                    tokens = lambda value: re.findall(r'"(?:\\.|[^"\\])*"|[()]|[^()\s]+', value)
                    self.assertEqual(tokens(path.read_text()), tokens(text))
                else:
                    self.assertEqual(path.read_text(), text)

    def test_invalid_manifests_create_no_partial_output(self):
        cases = [None, {}, "reader", [None], [{"path": "reader"}],
                 [{"path": "reader", "factory": "f();panic!()"}],
                 [{"path": "reader", "factory": "f"}] * 2,
                 [{"path": "reader", "factory": "f"}] * 65]
        for profiles in cases:
            with self.subTest(profiles=profiles):
                self.config["document_profiles"] = profiles
                with self.assertRaises(ValueError):
                    self.generate()
                self.assertFalse(self.output.exists())
        self.config["document_profiles"] = []
        with self.assertRaises(ValueError):
            self.generate()
        self.config["components"] = [None]
        with self.assertRaises(ValueError):
            self.generate()

    def test_changed_generated_file_is_preserved_without_partial_rewrite(self):
        self.config["document_profiles"] = [{"path": "reader", "factory": "profile"}]
        files = self.generate()
        (self.output / "registration.rs").write_text("application changes\n")
        with self.assertRaises(ValueError):
            self.generate()
        self.assertEqual((self.output / "registration.rs").read_text(), "application changes\n")
        self.assertEqual((self.output / "Cargo.toml").read_text(), files["Cargo.toml"])


if __name__ == "__main__":
    unittest.main()
