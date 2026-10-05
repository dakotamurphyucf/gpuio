"""Reject incomplete or misleading document qualification records without a GUI."""
import copy
import hashlib
from pathlib import Path
import re
import unittest

import measure_document_growth as m


def fixture():
    lines = []
    interactions = []
    def emit(name, payload):
        lines.append(m.PREFIX + name + ' ' + payload)
    def checkpoint(kind, document, index):
        number = len(interactions) + 1
        emit('checkpoint', f'({number} {kind} {document} {index})')
        item = dict(checkpoint=number, kind=kind, document=document, index=index, verified=True)
        if kind == 'navigate':
            item['fallback_notice'] = 'Rich view unavailable (ResourceLimit); showing source.'
            canonical = m.chunk(document, 1).encode()
            item['selection_copies'] = [dict(bytes=n, sha256=hashlib.sha256(canonical[:n]).hexdigest())
                                        for n in range(1, 9)]
            item['ready_page'] = dict(first=0, last=1024, total=index*m.CHUNK_BYTES,
                                      sha256=hashlib.sha256(m.chunk(document, 1).encode()[:1024]).hexdigest())
        else:
            text = ''.join(m.chunk(document, n) for n in range(1, index + 1)) if kind == 'copy' else m.SEED
            item['source_copy_sha256'] = hashlib.sha256(text.encode()).hexdigest()
            if kind == 'copy':
                raw, pages, first = text.encode(), [], 0
                while first < len(raw):
                    last = min(first + 65536, len(raw))
                    while True:
                        try:
                            selected = raw[first:last].decode()
                            break
                        except UnicodeDecodeError:
                            last -= 1
                    pages.append(dict(first=first, last=last, total=len(raw), sha256=hashlib.sha256(raw[first:last]).hexdigest()))
                    first = last
                item['pages'] = pages
                item['backward_pages'] = list(reversed(pages[:-1]))
                item['selection_copy_sha256'] = pages[-1]['sha256']
                item['end_line_copy_sha256'] = hashlib.sha256(selected.rsplit('\n', 1)[-1].encode()).hexdigest()
        interactions.append(item)
    def prepare(phase, count):
        values = ' '.join(f'({key} {count})' for key in sorted(m.PREPARATION_FIELDS))
        emit('preparation', f'({phase} (Document_preparation {values}))')
    def diagnostics(documents=0, source_bytes=0):
        from measure_list_history import ZERO_RESOURCES
        values = {key: 0 for key in ZERO_RESOURCES}
        values.update(documents=documents, document_source_bytes=source_bytes)
        result = ' '.join(f'({key} {value})' for key, value in values.items())
        return '(' + result + ' (native_command_queue ((commands 0)(bytes 0))) (scopes ((scopes 1)(tasks 1)(cleanups 0))))'
    emit('config', '(true 131072 (2 2 1))')
    checkpoint('initial', 0, 0)
    emit('begin', '(growth 10)')
    prepare('before', 1)
    for document, count in enumerate(m.sizes(True)):
        for index in range(1, count + 1):
            checkpoint('navigate', document, index)
            emit('append', f'({document} {index} {index*m.CHUNK_BYTES} 10 20)')
        checkpoint('copy', document, count)
    total = 5 * m.CHUNK_BYTES
    emit('retained', f'({total} {diagnostics(3, total)})')
    prepare('grown', 2)
    checkpoint('reset', 0, 0)
    checkpoint('profile-removed', 0, 0)
    prepare('after', 3)
    emit('finish', '(growth (Finished (elapsed_ns 1000000)(capture_ns 10)(dropped_inputs 0)(counts (1000 0 0 0 0))))')
    for metric in range(5):
        total, values = (1, '(1000 1000)') if metric == 0 else (0, '')
        emit('buckets', f'(growth (Buckets (metric {metric})(offset 0)(total {total})(values ({values}))))')
    emit('cleanup', diagnostics())
    emit('complete', str(5*m.CHUNK_BYTES))
    return '\n'.join(lines), interactions


class DocumentQualificationTest(unittest.TestCase):
    def test_paired_probe_fingerprint_matches_declared_wire_schema(self):
        root = Path(__file__).resolve().parents[1] / 'examples/performance_probe'
        digest = hashlib.sha256((root / 'schema.txt').read_bytes()).hexdigest()
        ocaml = (root / 'ocaml/gpuio_performance_probe.ml').read_text()
        rust = (root / 'rust/src/lib.rs').read_text()
        self.assertEqual(re.search(r'~fingerprint:"([a-f0-9]{64})"', ocaml)[1], digest)
        self.assertEqual(re.search(r'FINGERPRINT: &str = "([a-f0-9]{64})"', rust)[1], digest)
        self.assertIn('~version:2', ocaml)
        self.assertIn('version: 2,', rust)

    def test_fixture_is_exact_utf8_and_varies_by_identity(self):
        a, b = m.chunk(0, 1), m.chunk(1, 1)
        self.assertEqual(len(a.encode()), 128*1024)
        self.assertEqual(len(b.encode()), 128*1024)
        self.assertNotEqual(a, b)
        self.assertIn('λ 世界', a)
        self.assertEqual(sum(m.sizes(False))*m.CHUNK_BYTES, 20*1024**2)
        self.assertEqual(max(m.sizes(False))*m.CHUNK_BYTES, 8*1024**2)

    def test_complete_smoke_retains_all_phases_and_raw_histograms(self):
        output, interactions = fixture()
        result = m.validate(output, interactions, smoke=True)
        self.assertEqual(len(result['appends']), 5)
        self.assertEqual(len(result['preparation']), 3)
        self.assertEqual(result['interval']['histograms']['draw']['count'], 1000)
        self.assertEqual(m.budget_failures(result, 1000000), [])
        self.assertTrue(m.budget_failures(result, 2*1024**3))

    def test_rejects_missing_duplicate_stale_and_unverified_evidence(self):
        output, interactions = fixture()
        edits = [
            output.replace('GPUIO_DOCUMENT_PERF retained', 'MISSING retained'),
            output.replace('GPUIO_DOCUMENT_PERF append (0 2 262144', 'GPUIO_DOCUMENT_PERF append (0 1 131072', 1),
            output.replace('(source_bytes 2)', '(source_bytes 0)', 1),
            output + '\n' + m.PREFIX + 'complete 655360',
            output.replace('(counts (1000 0 0 0 0))', '(counts (999 0 0 0 0))'),
        ]
        for invalid in edits:
            with self.subTest(invalid=invalid[-100:]), self.assertRaises(ValueError):
                m.validate(invalid, interactions, smoke=True)
        for change in ('missing', 'unverified', 'copy', 'movement', 'selection-copy', 'end-line', 'pages', 'page-total', 'backward', 'page-copy', 'ready-page', 'fallback'):
            bad = copy.deepcopy(interactions)
            if change == 'missing':
                bad.pop()
            elif change == 'unverified':
                bad[0]['verified'] = False
            elif change == 'copy':
                bad[3]['source_copy_sha256'] = 'incorrect'
            elif change == 'pages':
                bad[3]['pages'].pop(0)
            elif change == 'page-total':
                bad[3]['pages'][0]['total'] = 0
            elif change == 'backward':
                bad[3]['backward_pages'].pop()
            elif change == 'ready-page':
                bad[1]['ready_page']['total'] = 0
            elif change == 'fallback':
                del bad[1]['fallback_notice']
            elif change == 'selection-copy':
                bad[1]['selection_copies'][0]['sha256'] = 'incorrect'
            elif change == 'end-line':
                bad[3]['end_line_copy_sha256'] = 'incorrect'
            elif change == 'page-copy':
                bad[3]['selection_copy_sha256'] = bad[3]['source_copy_sha256']
            else:
                bad[1]['selection_copies'] = bad[1]['selection_copies'][:1] * 8
            with self.subTest(change=change), self.assertRaises(ValueError):
                m.validate(output, bad, smoke=True)

    def test_budget_rejects_insufficient_frames_and_dropped_input(self):
        output, interactions = fixture()
        result = m.validate(output, interactions, smoke=True)
        result['interval']['histograms']['draw']['count'] = 999
        result['interval']['dropped_inputs'] = 1
        self.assertEqual(len(m.budget_failures(result, 1)), 2)


if __name__ == '__main__':
    unittest.main()
