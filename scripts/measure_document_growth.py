#!/usr/bin/env python3
"""Foreground macOS document growth, native navigation/copy and phase measurements.

Owns one GUI child, restores the original clipboard, and retains failures. The
20 MiB workload is three 8/8/4 MiB sources, not one oversized source. Preparation
counters are worker elapsed times, not CPU time or physical presentation.
"""
import argparse
import ctypes as C
import hashlib
import json
import math
from pathlib import Path
import platform
import re
import signal
import time

from measure_chart_stream import collect, command
from measure_list_history import fields, natural, read_interval, sexp
from measure_resource_lifecycle import retired

PREFIX = 'GPUIO_DOCUMENT_PERF '
TITLE = 'GPUIO · Growing document qualification'
CHUNK_BYTES = 128 * 1024
SEED = '# Qualification\n\nA **native** document · λ 世界.\n\n```ocaml\nlet answer = 42\n```\n'
PREPARATION_FIELDS = {'queue_us', 'configure_us', 'parse_us', 'highlight_us', 'search_us',
                      'source_bytes', 'completed', 'discarded', 'peak_workers', 'peak_reserved_bytes'}


def chunk(document, index):
    header = f'# Document {document} / chunk {index}\n\n'.encode()
    line = 'Native **Markdown** and `code` · λ 世界\n'.encode()
    count, remainder = divmod(CHUNK_BYTES - len(header), len(line))
    return (header + line * count + b' ' * remainder).decode()


def sizes(smoke):
    return [2, 2, 1] if smoke else [64, 64, 32]


def checkpoints(smoke):
    rows = [('initial', 0, 0)]
    for document, count in enumerate(sizes(smoke)):
        rows.extend(('navigate', document, index) for index in range(1, count + 1))
        rows.append(('copy', document, count))
    return rows + [('reset', 0, 0), ('profile-removed', 0, 0)]


def records(output):
    result = []
    for line in output.splitlines():
        if line.startswith(PREFIX):
            name, payload = line[len(PREFIX):].split(' ', 1)
            result.append((name, sexp(payload)))
    return result


def validate(output, interactions, *, smoke):
    rows = records(output)
    def take(name):
        if not rows or rows[0][0] != name:
            raise ValueError(f'Expected document record {name}')
        return rows.pop(0)[1]
    if take('config') != [str(smoke).lower(), str(CHUNK_BYTES), list(map(str, sizes(smoke)))]:
        raise ValueError('Document workload configuration mismatch')
    preparation = []
    def prepare(phase):
        name, value = take('preparation')
        if name != phase or value[0] != 'Document_preparation':
            raise ValueError('Missing preparation snapshot')
        value = fields(value[1:])
        if set(value) != PREPARATION_FIELDS:
            raise ValueError('Unexpected preparation fields')
        value = {k: natural(v) for k, v in value.items()}
        if preparation and any(value[k] < preparation[-1][k] for k in value):
            raise ValueError('Preparation counters regressed')
        preparation.append(value)
    expected = checkpoints(smoke)
    position = 0
    def checkpoint(kind, document, index):
        nonlocal position
        if take('checkpoint') != [str(position + 1), kind, str(document), str(index)]:
            raise ValueError('Missing, reordered or duplicate interaction checkpoint')
        if position >= len(interactions):
            raise ValueError('Native interaction evidence missing')
        item = interactions[position]
        if (item['kind'], item['document'], item['index']) != expected[position]:
            raise ValueError('Native interaction disagrees with application checkpoint')
        if item['checkpoint'] != position + 1 or item['verified'] is not True:
            raise ValueError('Unverified native interaction')
        if kind == 'navigate':
            selections = item.get('selection_copies', [])
            canonical = chunk(document, 1).encode()
            if len(selections) != 8:
                raise ValueError('Missing native selection movement evidence')
            previous = 0
            for selection in selections:
                length = selection.get('bytes')
                if type(length) is not int or not previous < length <= len(canonical):
                    raise ValueError('Native selection did not expand from the source start')
                if selection.get('sha256') != hashlib.sha256(canonical[:length]).hexdigest():
                    raise ValueError('Native selection copy differs from source prefix')
                previous = length
        else:
            text = (''.join(chunk(document, n) for n in range(1, index + 1))
                    if kind == 'copy' else SEED)
            digest = hashlib.sha256(text.encode()).hexdigest()
            if item.get('source_copy_sha256') != digest:
                raise ValueError('Source copy did not match canonical bytes')
            if kind == 'copy':
                canonical = text.encode()
                pages = item.get('pages', [])
                end = 0
                for page in pages:
                    first, last = page['first'], page['last']
                    if (type(first) is not int or type(last) is not int or first != end
                            or page.get('total') != len(canonical)
                            or not first < last <= min(first + 65536, len(canonical))):
                        raise ValueError('Noncontiguous or out-of-bounds native page coverage')
                    if page['sha256'] != hashlib.sha256(canonical[first:last]).hexdigest():
                        raise ValueError('Native page differs from canonical source')
                    end = last
                if end != len(canonical) or item.get('backward_pages') != list(reversed(pages[:-1])):
                    raise ValueError('Incomplete forward/backward source traversal')
                selected = canonical[pages[-1]['first']:pages[-1]['last']].decode()
                if item.get('selection_copy_sha256') != hashlib.sha256(selected.encode()).hexdigest():
                    raise ValueError('Native selection copy differs from the current page')
                last_line = selected.rsplit('\n', 1)[-1]
                if item.get('end_line_copy_sha256') != hashlib.sha256(last_line.encode()).hexdigest():
                    raise ValueError('Native end navigation did not select the final line')
        position += 1
    checkpoint('initial', 0, 0)
    name, capture = take('begin')
    if name != 'growth':
        raise ValueError('Wrong interval')
    prepare('before')
    appends = []
    for document, count in enumerate(sizes(smoke)):
        for index in range(1, count + 1):
            checkpoint('navigate', document, index)
            d, i, length, publication, ready = take('append')
            if list(map(natural, (d, i, length))) != [document, index, index * CHUNK_BYTES]:
                raise ValueError('Incomplete or incorrectly sized document growth')
            page = interactions[position - 1].get('ready_page', {})
            first, last = page.get('first'), page.get('last')
            if (first != 0 or type(last) is not int or not 0 < last <= 65536
                    or page.get('total') != index * CHUNK_BYTES):
                raise ValueError('Missing current installed-page readiness observation')
            if interactions[position - 1].get('fallback_notice') != 'Rich view unavailable (ResourceLimit); showing source.':
                raise ValueError('Missing resource-limit source fallback observation')
            if page.get('sha256') != hashlib.sha256(chunk(document, 1).encode()[:last]).hexdigest():
                raise ValueError('Ready page differs from canonical source')
            appends.append(dict(document=document, index=index, source_bytes=natural(length),
                                publication_ns=natural(publication),
                                publication_to_ax_ready_ack_ns=natural(ready)))
        if not smoke and document < 2 and natural(take('limit-rejected')) != document:
            raise ValueError('Missing 8 MiB limit rejection')
        checkpoint('copy', document, count)
    total = sum(sizes(smoke)) * CHUNK_BYTES
    count, retained = take('retained')
    retained = fields(retained)
    if natural(count) != total or natural(retained['documents']) != 3:
        raise ValueError('Expected three retained sources')
    # The logical source accounting can include staged/accepted snapshots; the
    # exact live canonical size is checked separately by the application.
    if natural(retained['document_source_bytes']) < total:
        raise ValueError('Retained-source charge below canonical source size')
    prepare('grown')
    checkpoint('reset', 0, 0)
    checkpoint('profile-removed', 0, 0)
    prepare('after')
    interval = read_interval(take, 'growth', wall_clock=False, before_capture=natural(capture))
    cleanup = take('cleanup')
    retired(cleanup)
    if natural(take('complete')) != total or rows or position != len(interactions):
        raise ValueError('Incomplete workload or extra evidence')
    cumulative = PREPARATION_FIELDS - {'peak_workers', 'peak_reserved_bytes'}
    preparation_deltas = {
        phase: {key: after[key] - before[key] for key in cumulative}
        for phase, before, after in zip(('growth', 'reset_and_removal'), preparation, preparation[1:])
    }
    return dict(appends=appends, preparation=preparation,
                preparation_deltas=preparation_deltas, retained=retained,
                interval=interval, cleanup=fields(cleanup), total_source_bytes=total)


def budget_failures(workload, peak_rss):
    draw = workload['interval']['histograms']['draw']
    failures = []
    if draw['count'] < 1000:
        failures.append('Fewer than 1000 native draw samples')
    if draw['p95'] is None or draw['p95'] > 16_700_000:
        failures.append('Native draw p95 exceeds 16.7 ms')
    if draw['p99'] is None or draw['p99'] > 33_400_000:
        failures.append('Native draw p99 exceeds 33.4 ms')
    if peak_rss > 1536 * 1024**2:
        failures.append('Peak RSS exceeds 1.5 GiB')
    if workload['interval']['dropped_inputs']:
        failures.append('Native input timestamps were dropped')
    return failures


class Checkpoints:
    def __init__(self, log, interactions, board, *, smoke):
        self.log, self.interactions, self.board = log, interactions, board
        self.expected = checkpoints(smoke)
        self.offset, self.pending, self.mac = 0, b'', None
        self.sources = [bytearray() for _ in range(3)]

    def close(self):
        if self.mac:
            self.mac.release(self.mac.app)
            self.mac = None

    def clipboard_sequence(self):
        return self.board.call(self.board.board, 'changeCount', result=C.c_long)

    def copied(self, text, previous_sequence):
        deadline = time.monotonic() + 10
        while self.clipboard_sequence() == previous_sequence or self.board.text() != text:
            if time.monotonic() > deadline:
                raise RuntimeError('Native copied text differs from canonical source')
            time.sleep(.025)
        return hashlib.sha256(text.encode()).hexdigest()

    def copy_prefix(self, canonical, previous_bytes):
        sequence = self.clipboard_sequence()
        self.mac.key(8, 1 << 20)
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if self.clipboard_sequence() != sequence:
                text = self.board.text()
                if text is not None:
                    raw = text.encode()
                    if previous_bytes < len(raw) and canonical.startswith(raw):
                        return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            time.sleep(.01)
        raise RuntimeError('Native selection copy did not expand over canonical source')

    def page(self, canonical, first):
        mac = self.mac
        deadline = time.monotonic() + 10
        marker = None
        match = None
        while time.monotonic() < deadline:
            marker = mac.find(TITLE, f'Source bytes {first}–', 'AXStaticText', contains=True)
            if marker:
                try:
                    label = ' '.join(mac.text(marker, name) or '' for name in ('AXValue', 'AXTitle', 'AXDescription'))
                finally:
                    mac.release(marker)
                candidate = re.search(r'Source bytes (\d+)–(\d+) of (\d+)', label)
                if candidate and int(candidate[3]) == len(canonical):
                    match = candidate
                    break
            time.sleep(.025)
        if not match:
            self.capture_failure()
            raise RuntimeError('Installed source-page byte interval was not accessible; see failure artifacts')
        start, end, total = map(int, match.groups())
        if start != first or total != len(canonical) or not start < end <= min(start + 65536, total):
            raise RuntimeError('Unexpected native source-page bounds')
        expected = canonical[start:end].decode()
        node = mac.wait_find(TITLE, 'Qualification document', 'AXTextArea')
        try:
            deadline = time.monotonic() + 10
            while mac.text(node, 'AXValue') != expected:
                if time.monotonic() > deadline:
                    raise RuntimeError('Native page text differs from its canonical byte interval')
                time.sleep(.01)
        finally:
            mac.release(node)
        return dict(first=start, last=end, total=total, sha256=hashlib.sha256(expected.encode()).hexdigest())

    def capture_failure(self):
        """Bounded diagnostics of this owned fixture window, only after failure."""
        mac, rows = self.mac, []
        deadline = time.monotonic() + 3
        def visit(node, depth):
            if depth > 16 or len(rows) >= 200 or time.monotonic() > deadline:
                return
            values, children = mac.node_values(node)
            rows.append(dict(depth=depth, values=[v[:200] for v in values]))
            try:
                for child in children:
                    visit(child, depth + 1)
            finally:
                for child in children:
                    mac.release(child)
        root = mac.window(TITLE)
        if root:
            try:
                visit(root, 0)
            finally:
                mac.release(root)
        (self.log.parent / 'failure-ax.json').write_text(json.dumps(rows, indent=2) + '\n')
        from test_canvas import screenshot
        screenshot(mac, self.log.parent / 'failure-window.png', title=TITLE)

    def exercise(self, kind, document, index):
        mac = self.mac
        evidence = dict(kind=kind, document=document, index=index)
        if kind in ('navigate', 'copy'):
            if kind == 'copy':
                text = ''.join(chunk(document, n) for n in range(1, index + 1))
                canonical = text.encode()
                pages = [self.page(canonical, 0)]
                while pages[-1]['last'] < len(canonical):
                    mac.press(TITLE, 'Next source page')
                    pages.append(self.page(canonical, pages[-1]['last']))
                evidence['pages'] = pages
                selected_text = canonical[pages[-1]['first']:pages[-1]['last']].decode()
            node = mac.wait_find(TITLE, 'Qualification document', 'AXTextArea')
            try:
                mac.set(node, 'AXFocused', mac.true)
                if kind == 'navigate':
                    # Shift-arrow moves a real native range and drives redraws;
                    # large PageDown/PageUp steps also exercise viewport reveal.
                    mac.key(126, 1 << 20)  # Command-Up, known first source position.
                    time.sleep(.03)
                    changes = []
                    previous_bytes = 0
                    for _ in range(8):
                        mac.key(125, 1 << 17)
                        time.sleep(.04)
                        copied = self.copy_prefix(self.sources[document], previous_bytes)
                        changes.append(copied)
                        previous_bytes = copied['bytes']
                    mac.key(121)
                    time.sleep(.05)
                    mac.key(116)
                    time.sleep(.05)
                    evidence['selection_copies'] = changes
                else:
                    mac.key(125, 1 << 20)  # Command-Down, native last page position.
                    time.sleep(.1)
                    mac.key(123, (1 << 20) | (1 << 17))  # Select from end to beginning of last line.
                    time.sleep(.05)
                    before = self.clipboard_sequence()
                    mac.key(8, 1 << 20)
                    evidence['end_line_copy_sha256'] = self.copied(selected_text.rsplit('\n', 1)[-1], before)
                    mac.key(0, 1 << 20)  # Native select-all and copy.
                    time.sleep(.05)
                    before = self.clipboard_sequence()
                    mac.key(8, 1 << 20)
                    evidence['selection_copy_sha256'] = self.copied(selected_text, before)
                    before = self.clipboard_sequence()
                    mac.press(TITLE, 'Copy source')
                    evidence['source_copy_sha256'] = self.copied(text, before)
            finally:
                mac.release(node)
            if kind == 'copy':
                backward = []
                for page in reversed(pages[:-1]):
                    mac.press(TITLE, 'Previous source page')
                    backward.append(self.page(canonical, page['first']))
                evidence['backward_pages'] = backward
        else:
            if kind in ('initial', 'reset'):
                mac.release(mac.wait_find(TITLE, 'Inspect highlighted code', 'AXButton'))
            elif kind == 'profile-removed':
                deadline = time.monotonic() + 10
                while True:
                    node = mac.find(TITLE, 'Inspect highlighted code', 'AXButton')
                    if not node:
                        break
                    mac.release(node)
                    if time.monotonic() > deadline:
                        raise RuntimeError('Retired profile action still accessible')
                    time.sleep(.025)
                mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
            else:
                raise ValueError('Unknown document checkpoint')
            before = self.clipboard_sequence()
            mac.press(TITLE, 'Copy source')
            evidence['source_copy_sha256'] = self.copied(SEED, before)
        evidence['verified'] = True
        return evidence

    def __call__(self, child):
        with self.log.open('rb') as stream:
            stream.seek(self.offset)
            data = stream.read(1024 * 1024)
            self.offset += len(data)
        lines = (self.pending + data).split(b'\n')
        self.pending = lines.pop()
        if len(self.pending) > 65536:
            raise ValueError('Oversized document record')
        for line in lines:
            prefix = (PREFIX + 'checkpoint ').encode()
            if not line.startswith(prefix):
                continue
            number, kind, document, index = sexp(line[len(prefix):].decode())
            number, document, index = map(natural, (number, document, index))
            if number != len(self.interactions) + 1 or number > len(self.expected):
                raise ValueError('Reordered or duplicate checkpoint')
            if (kind, document, index) != self.expected[number - 1]:
                raise ValueError('Unexpected native interaction')
            if self.mac is None:
                from test_agent_chat import Mac
                self.mac = Mac(child.pid, child)
            ready_page = None
            if kind == 'navigate':
                self.sources[document].extend(chunk(document, index).encode())
                ready_page = self.page(self.sources[document], 0)
                notice = 'Rich view unavailable (ResourceLimit); showing source.'
                self.mac.release(self.mac.wait_find(TITLE, notice, 'AXStaticText'))
                child.stdin.write(f'ready {number}\n'.encode())
                child.stdin.flush()
            evidence = dict(checkpoint=number, **self.exercise(kind, document, index))
            if ready_page is not None:
                evidence['ready_page'] = ready_page
                evidence['fallback_notice'] = notice
            self.interactions.append(evidence)
            child.stdin.write(f'continue {number}\n'.encode())
            child.stdin.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/examples/performance_document/main.exe'))
    parser.add_argument('--build-profile', choices=('dev', 'release'), required=True)
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--check-budgets', action='store_true')
    parser.add_argument('--timeout', type=float, default=1200)
    args = parser.parse_args()
    if platform.system() != 'Darwin':
        parser.error('Native keyboard/clipboard qualification currently requires macOS')
    if not math.isfinite(args.timeout) or not 0 < args.timeout <= 3600:
        parser.error('Timeout must be finite and in (0,3600]')
    if args.check_budgets and (args.smoke or args.build_profile != 'release'):
        parser.error('Budget checks require a full optimized workload')
    args.output.mkdir(parents=True, exist_ok=False)
    log = args.output / 'application.log'
    report = dict(complete=False, interactions=[], smoke=args.smoke, build_profile=args.build_profile,
                  platform=platform.platform(), architecture=platform.machine(),
                  measurement='Native submitted frames, worker elapsed counters, publication/AX readiness observations; not physical presentation')
    def interrupted(signum, _frame):
        raise SystemExit(128 + signum)
    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        with args.executable.open('rb') as binary:
            report['executable_sha256'] = hashlib.file_digest(binary, 'sha256').hexdigest()
        report.update(revision=command('git', 'rev-parse', 'HEAD'), dirty=bool(command('git', 'status', '--porcelain')),
                      hardware=command('sysctl', '-n', 'machdep.cpu.brand_string'),
                      memory_bytes=int(command('sysctl', '-n', 'hw.memsize')),
                      display=command('system_profiler', 'SPDisplaysDataType', '-json'),
                      power=command('pmset', '-g', 'batt'), thermal=command('pmset', '-g', 'therm'))
        from mac_clipboard import preserved_clipboard
        with preserved_clipboard() as board:
            driver = Checkpoints(log, report['interactions'], board, smoke=args.smoke)
            try:
                collect(args.executable, log, report, args.timeout,
                        arguments=['--smoke'] if args.smoke else [], on_poll=driver)
            finally:
                driver.close()
        report['clipboard_restored'] = True
        report['workload'] = validate(log.read_text(), report['interactions'], smoke=args.smoke)
        if args.check_budgets:
            report['budget_failures'] = budget_failures(report['workload'], report['peak_rss_bytes'])
            if report['budget_failures']:
                raise RuntimeError('; '.join(report['budget_failures']))
        report['complete'] = True
    except BaseException as error:
        report['failure'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    print(f'DOCUMENT_GROWTH_MEASUREMENT_OK report={args.output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
