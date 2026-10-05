"""Standalone manifests must preserve pinned workspace dependency semantics."""
import tomllib
import unittest

from vendor_gpui import standalone_manifest


class StandaloneManifest(unittest.TestCase):
    def test_macos_inheritance_preserves_pins_and_merges_features(self):
        workspace = dict(package=dict(edition='2024', publish=False), dependencies=dict(
            gpui=dict(path='crates/gpui', default_features=False, features=['a']),
            cocoa=dict(version='0.26', features=['base'])))
        # Cargo spells this field with a dash; no rewriting is permitted.
        workspace['dependencies']['gpui']['default-features'] = workspace['dependencies']['gpui'].pop('default_features')
        source = dict(url='https://example.test/pinned.git', commit='abc123')
        original = '''[package]
name = "gpui_macos"
edition.workspace = true
publish.workspace = true
[lints]
workspace = true
[dependencies]
gpui.workspace = true
[target.'cfg(target_os = "macos")'.dependencies]
cocoa = { workspace = true, features = ["base", "extra"], optional = true }
'''
        result = tomllib.loads(standalone_manifest(original, workspace, source))
        self.assertEqual(result['package']['edition'], '2024')
        self.assertIs(result['package']['publish'], False)
        self.assertEqual(result['dependencies']['gpui'], dict(
            git=source['url'], rev='abc123', features=['a'], **{'default-features': False}))
        self.assertEqual(result['target']['cfg(target_os = "macos")']['dependencies']['cocoa'],
                         dict(version='0.26', features=['base', 'extra'], optional=True))
        self.assertNotIn('workspace', result['lints'])

    def test_unrecognized_inheritance_is_not_silently_accepted(self):
        with self.assertRaises(ValueError):
            standalone_manifest('[package]\nlicense.workspace = true\n',
                                {'package': {}, 'dependencies': {}}, {})


if __name__ == '__main__':
    unittest.main()
