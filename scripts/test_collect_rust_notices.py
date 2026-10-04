#!/usr/bin/env python3
"""Offline notice provenance and graph tests; no Cargo/network/build required."""
import json
import hashlib
from pathlib import Path
import tempfile
import unittest

from collect_rust_notices import collect, dependency_closure, digest, load_supplemental, notice_files


def package(root, name, identity=None):
    directory = root / (identity or name)
    directory.mkdir()
    manifest = directory / 'Cargo.toml'
    manifest.write_text('[package]\n')
    return {'id': identity or name, 'name': name, 'version': '1.0.0',
            'source': None, 'manifest_path': str(manifest),
            'license': 'MIT OR Apache-2.0', 'license_file': None}


def metadata(packages, dependencies=None):
    return {'packages': packages, 'resolve': {'nodes': [
        {'id': p['id'], 'deps': (dependencies or {}).get(p['id'], [])} for p in packages]}}


def edge(identity, *kinds):
    return {'pkg': identity, 'dep_kinds': [{'kind': k} for k in kinds]}


class Closure(unittest.TestCase):
    def test_build_dependencies_cycles_and_mixed_dev_edges(self):
        packages = [{'id': n, 'name': n} for n in ['app', 'normal', 'build', 'dev', 'mixed']]
        result = dependency_closure(metadata(packages, {
            'app': [edge('normal', None), edge('build', 'build'), edge('dev', 'dev'),
                    edge('mixed', 'dev', None)],
            'normal': [edge('app', None)]}), ['app'])
        self.assertEqual([p['id'] for p in result], ['app', 'build', 'mixed', 'normal'])

    def test_ambiguous_roots_missing_nodes_and_unknown_kinds_fail(self):
        with self.assertRaises(ValueError):
            dependency_closure(metadata([{'id': 'a', 'name': 'same'},
                                         {'id': 'b', 'name': 'same'}]), ['same'])
        packages = [{'id': 'a', 'name': 'app'}]
        for dependencies in [[edge('absent', None)], [edge('a', 'unknown')], [edge('a')]]:
            with self.subTest(dependencies=dependencies), self.assertRaises(ValueError):
                dependency_closure(metadata(packages, {'a': dependencies}), ['app'])


class NoticeCollection(unittest.TestCase):
    def test_preserves_identity_expression_bytes_and_nested_notices(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = package(root, 'same', 'registry-one')
            second = package(root, 'same', 'registry-two')
            for p, payload in [(first, b'Copyright A\r\n'), (second, b'Copyright B\n')]:
                directory = Path(p['manifest_path']).parent
                (directory / 'licenses').mkdir()
                (directory / 'licenses/MIT.txt').write_bytes(payload)
                (directory / 'ACKNOWLEDGEMENTS.md').write_bytes(b'Asset notice\n')
            app = package(root, 'app')
            output = root / 'output'
            report = collect(metadata([app, first, second], {
                'app': [edge('registry-one', None), edge('registry-two', None)]}), ['app'], output)
            rows = [p for p in report['packages'] if p['name'] == 'same']
            self.assertEqual(len(rows), 2)
            self.assertFalse(report['license_review_complete'])
            self.assertEqual(report['packages_with_review_issues'], 1)
            self.assertNotEqual(rows[0]['notices'][0]['file'], rows[1]['notices'][0]['file'])
            for row in rows:
                self.assertEqual(row['license_expression'], 'MIT OR Apache-2.0')
                self.assertEqual(len(row['notices']), 2)
                directory = Path(row['manifest_path']).parent
                for notice in row['notices']:
                    copied = (output / notice['file']).read_bytes()
                    self.assertEqual(copied, (directory / notice['source_path']).read_bytes())
                    self.assertEqual(notice['sha256'], digest(copied))
            json.dumps(report)
            with self.assertRaises(FileExistsError):
                collect(metadata([app]), ['app'], output)

    def test_missing_external_and_symlink_notices_remain_unresolved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            p = package(root, 'nested')
            directory = Path(p['manifest_path']).parent
            (root / 'LICENSE').write_text('Different parent license')
            p['license_file'] = '../LICENSE'
            (directory / 'LICENSE').symlink_to(root / 'LICENSE')
            _, files, issues = notice_files(p)
            self.assertEqual(files, [])
            self.assertTrue(any('outside package' in issue for issue in issues))
            self.assertTrue(any('symlink notice' in issue for issue in issues))
            self.assertTrue(any('no package-local' in issue for issue in issues))
            p['license_file'] = 'missing.txt'
            _, _, issues = notice_files(p)
            self.assertTrue(any('missing declared' in issue for issue in issues))

    def test_explicit_nonstandard_filename_and_empty_text(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            p = package(root, 'lib')
            directory = Path(p['manifest_path']).parent
            (directory / 'terms.txt').write_bytes(b'')
            p['license_file'] = 'terms.txt'
            report = collect(metadata([p]), ['lib'], root / 'output')
            self.assertEqual(len(report['packages'][0]['notices']), 1)
            self.assertTrue(any('empty notice' in issue
                                for issue in report['packages'][0]['review_issues']))


class SupplementalTexts(unittest.TestCase):
    def test_checked_in_notice_bytes_and_upstream_blob_identities(self):
        root = Path(__file__).resolve().parents[1]
        loaded = load_supplemental(root / 'third_party/notice-sources.json', root)
        self.assertTrue(loaded['entries'])
        for entry, files in loaded['entries']:
            for item, data in files:
                if 'git_blob_sha1' in item:
                    with self.subTest(package=entry['package']['name'], path=item['path']):
                        git_blob = b'blob ' + str(len(data)).encode() + b'\0' + data
                        self.assertEqual(hashlib.sha1(git_blob).hexdigest(), item['git_blob_sha1'])

    def prepare(self, root):
        p = package(root, 'nested')
        (root / 'LICENSE').write_bytes(b'Exact upstream license\r\n')
        entry = {'package': {k: p[k] for k in ('name', 'version', 'source')},
                 'rationale': 'Exact pinned workspace attribution',
                 'files': [{'path': 'LICENSE', 'sha256': digest((root / 'LICENSE').read_bytes()),
                            'provenance': 'pinned upstream revision/license'}]}
        entry['package'].update({'license_expression': p['license'],
                                 'manifest_sha256': digest(Path(p['manifest_path']).read_bytes())})
        path = root / 'sources.json'
        path.write_text(json.dumps({'schema_version': 1, 'entries': [entry]}))
        return p, entry, path

    def test_pinned_text_resolves_missing_bytes_without_blessing_license(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            p, _, path = self.prepare(root)
            supplemental = load_supplemental(path, root)
            report = collect(metadata([p]), ['nested'], root / 'output', supplemental)
            row = report['packages'][0]
            self.assertFalse(report['license_review_complete'])
            self.assertEqual(report['packages_without_collected_text'], 0)
            self.assertEqual(report['packages_with_supplemental_text'], 1)
            self.assertTrue(row['review_issues'])  # Original discovery gaps remain visible.
            self.assertEqual(row['license_expression'], 'MIT OR Apache-2.0')
            notice = row['supplemental_notices'][0]
            self.assertEqual((root / 'output' / notice['file']).read_bytes(),
                             (root / 'LICENSE').read_bytes())
            self.assertEqual(supplemental['raw'], path.read_bytes())

    def test_manifest_and_expression_drift_fail_before_creating_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            p, _, path = self.prepare(root)
            supplemental = load_supplemental(path, root)
            p['license'] = 'GPL-3.0'
            with self.assertRaisesRegex(ValueError, 'stale supplemental'):
                collect(metadata([p]), ['nested'], root / 'output', supplemental)
            self.assertFalse((root / 'output').exists())
            p['license'] = 'MIT OR Apache-2.0'
            Path(p['manifest_path']).write_text('changed manifest')
            with self.assertRaisesRegex(ValueError, 'stale supplemental'):
                collect(metadata([p]), ['nested'], root / 'output', supplemental)
            self.assertFalse((root / 'output').exists())

    def test_identity_changes_do_not_receive_an_old_attribution(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            p, _, path = self.prepare(root)
            supplemental = load_supplemental(path, root)
            p['source'] = 'registry+other'
            report = collect(metadata([p]), ['nested'], root / 'output', supplemental)
            self.assertEqual(report['packages_with_supplemental_text'], 0)
            self.assertEqual(report['packages_without_collected_text'], 1)
            self.assertEqual(len(report['unused_supplemental_packages']), 1)

    def test_text_hash_path_symlink_and_duplicate_admission(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, entry, path = self.prepare(root)
            (root / 'link').symlink_to(root / 'LICENSE')
            for filename in ['../LICENSE', '/etc/passwd', 'link']:
                entry['files'][0]['path'] = filename
                path.write_text(json.dumps({'schema_version': 1, 'entries': [entry]}))
                with self.subTest(filename=filename), self.assertRaises(ValueError):
                    load_supplemental(path, root)
            entry['files'][0]['path'] = 'LICENSE'
            path.write_text(json.dumps({'schema_version': 1, 'entries': [entry, entry]}))
            with self.assertRaisesRegex(ValueError, 'duplicate supplemental'):
                load_supplemental(path, root)
            path.write_text(json.dumps({'schema_version': 1, 'entries': [entry]}))
            (root / 'LICENSE').write_bytes(b'Different contents')
            with self.assertRaisesRegex(ValueError, 'hash/contents mismatch'):
                load_supplemental(path, root)


if __name__ == '__main__':
    unittest.main()
