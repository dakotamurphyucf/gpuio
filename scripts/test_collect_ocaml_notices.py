#!/usr/bin/env python3
"""Portable tests for notice provenance and explicit-switch isolation."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import collect_ocaml_notices as collect


class NoticeTests(unittest.TestCase):
    def test_installed_list_is_strict_and_does_not_accept_paths_or_duplicates(self):
        self.assertEqual(collect.installed_packages(b'b |v0.17.0\na|5.3.0\n'), [('a', '5.3.0'), ('b', 'v0.17.0')])
        for raw in [b'', b'a|1\na|2', b'../a|1', b'a|../1', b'a|1|MIT']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                collect.installed_packages(raw)

    def test_exact_bytes_metadata_and_nested_notices_survive_collection(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source'
            (source / 'assets').mkdir(parents=True)
            manifest = source / 'opam'
            manifest.write_bytes(b'license: "MIT"\n')
            content = b'Copyright\r\npermission\n\xff'
            (source / 'assets/LICENSE-asset').write_bytes(content)
            (source / 'README').write_text('unrelated')
            (source / 'licenses').mkdir()
            (source / 'licenses/bundled.txt').write_text('nested dependency terms')
            row, files = collect.package_record('installed', 'a', '1', manifest, '"MIT"', [('source-tree', source)])
            output = root / 'output'
            report = collect.write_inventory(output, [row], files, {'license_review_complete': True, 'schema_version': 99})
            self.assertFalse(report['license_review_complete'])
            self.assertEqual(report['schema_version'], 1)
            self.assertEqual(report['packages_without_collected_text'], 0)
            self.assertEqual(len(row['notices']), 2)
            self.assertEqual(row['notices'][1]['source_path'], 'licenses/bundled.txt')
            notice = row['notices'][0]
            self.assertEqual((output / notice['file']).read_bytes(), content)
            self.assertEqual(notice['sha256'], collect.digest(content))
            self.assertEqual((output / row['manifest_file']).read_bytes(), manifest.read_bytes())
            self.assertEqual(notice['source_path'], 'assets/LICENSE-asset')
            manifest.write_text('license: "changed"\n')
            self.assertNotEqual(row['manifest_sha256'], collect.digest(manifest.read_bytes()))

    def test_symlinks_missing_and_empty_files_remain_visible_review_gaps(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source'
            source.mkdir()
            outside = root / 'outside'
            outside.mkdir()
            (outside / 'LICENSE').write_text('must not be copied')
            (source / 'linked').symlink_to(outside, target_is_directory=True)
            (source / 'LICENSE').symlink_to(outside / 'LICENSE')
            (source / 'NOTICE').write_bytes(b'')
            manifest = source / 'opam'
            manifest.write_text('opam-version: "2.0"')
            row, files = collect.package_record('installed', 'a', '1', manifest, '',
                                               [('source-tree', source), ('installed-doc', root / 'absent')])
            self.assertEqual(row['notices'], [])
            self.assertEqual(len(files), 1)
            issues = '\n'.join(row['review_issues'])
            for expected in ['symlink directory', 'symlink notice', 'empty notice', 'missing source directory', 'missing declared license', 'no collected notice']:
                self.assertIn(expected, issues)
            manifest.unlink()
            manifest.symlink_to(outside / 'LICENSE')
            with self.assertRaises(ValueError):
                collect.package_record('installed', 'a', '1', manifest, '', [])

    def test_invalid_or_existing_destinations_never_create_partial_inventory(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'out'
            for files in [[('../escape', b'x')], [('/absolute', b'x')], [('same', b'1'), ('same', b'2')]]:
                with self.assertRaises(ValueError):
                    collect.write_inventory(output, [], files, {})
                self.assertFalse(output.exists())
            output.mkdir()
            sentinel = output / 'existing'
            sentinel.write_text('keep')
            with self.assertRaises(ValueError):
                collect.write_inventory(output, [], [], {})
            self.assertEqual(sentinel.read_text(), 'keep')

    def test_cli_uses_only_explicit_read_only_switch_commands_and_keeps_vendor_pins(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prefix = root / 'opam/project'
            manifest = prefix / '.opam-switch/packages/example.1/opam'
            manifest.parent.mkdir(parents=True)
            manifest.write_text('opam-version: "2.0"\nlicense: "MIT"\n')
            documentation = prefix / 'doc/example'
            documentation.mkdir(parents=True)
            (documentation / 'LICENSE').write_text('installed bytes')
            project = root / 'repo'
            vendor = project / 'vendor/local'
            vendor.mkdir(parents=True)
            (vendor / 'local.opam').write_bytes(manifest.read_bytes())
            (vendor / 'LICENSE').write_text('vendor bytes')
            for relative in ['gpuio.opam', 'gpuio.opam.locked', 'dune-project', 'third_party/sources.json']:
                path = project / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('pinned ' + relative)
            commands = []
            listings = [b'example |1\n']

            def run(argv, **_):
                commands.append(argv)
                if argv == ['opam', '--version']:
                    return SimpleNamespace(stdout=b'2.3.0\n')
                self.assertIn('--root', argv)
                self.assertEqual(argv[argv.index('--root') + 1], str((root / 'opam').resolve()))
                self.assertEqual(argv[argv.index('--switch') + 1], 'project')
                if argv[1] == 'var':
                    return SimpleNamespace(stdout=(str(prefix) + '\n').encode())
                if argv[1] == 'list':
                    return SimpleNamespace(stdout=listings.pop(0) if len(listings) > 1 else listings[0])
                self.assertEqual(argv[1], 'show')
                self.assertIn('--just-file', argv)
                return SimpleNamespace(stdout=b'"MIT"\n')

            output = root / 'result'
            argv = ['collect', '--opam-root', str(root / 'opam'), '--switch', 'project',
                    '--source-root', str(project), '--vendor', 'vendor/local', '--output', str(output)]
            with patch('sys.argv', argv), patch.object(collect.subprocess, 'run', side_effect=run), contextlib.redirect_stdout(io.StringIO()):
                collect.main()
            result = json.loads((output / 'inventory.json').read_text())
            self.assertEqual([p['kind'] for p in result['packages']], ['installed', 'vendor'])
            self.assertEqual(result['switch_prefix'], str(prefix))
            self.assertEqual(result['packages_without_collected_text'], 0)
            for entry in result['source_inputs']:
                self.assertEqual(collect.digest((output / entry['file']).read_bytes()), entry['sha256'])
            self.assertEqual({c[1] for c in commands}, {'var', 'list', 'show', '--version'})
            # A concurrent switch change cannot produce an apparently stable
            # inventory with mixed installed versions or overwrite the first one.
            listings[:] = [b'example |1\n', b'example |2\n']
            second = root / 'changed-result'
            argv[-1] = str(second)
            with patch('sys.argv', argv), patch.object(collect.subprocess, 'run', side_effect=run), self.assertRaisesRegex(ValueError, 'switch changed'):
                collect.main()
            self.assertFalse(second.exists())
            self.assertEqual(json.loads((output / 'inventory.json').read_text()), result)


if __name__ == '__main__':
    unittest.main()
