"""Strict resource-only records from the opt-in native presentation audit.

Settled zero/missing/invalid-clock outcomes are retained, not counted as timing
success. Timing acceptance belongs to presentation_report and its workload gates.
"""
import json

PREFIX = 'GPUIO_PRESENTATION_AUDIT '
COUNTS = ('attempted', 'admitted', 'presented', 'zero', 'not_submitted', 'missing',
          'invalid_clock', 'saturated', 'duplicate_callbacks', 'histogram_overflow',
          'trace_truncated')


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate presentation retirement field')
        result[key] = value
    return result


def record(line):
    if not line.startswith(PREFIX):
        raise ValueError('Missing presentation retirement prefix')
    row = json.loads(line[len(PREFIX):], object_pairs_hook=_unique_object)
    integers = ('schema', 'cycle', 'session', 'window', 'pending', *COUNTS)
    if (not isinstance(row, dict) or set(row) != {*integers, 'accepting', 'window_closed'}
            or any(type(row[k]) is not int or not 0 <= row[k] < 2**64 for k in integers)):
        raise ValueError('Malformed presentation retirement record')
    if (row['schema'] != 1 or not 1 <= row['cycle'] <= 33
            or not row['session'] or not row['window']
            or row['accepting'] is not False or row['window_closed'] is not True
            or row['pending'] != 0):
        raise ValueError('Presentation owner was not closed, stopped and settled')
    settled = sum(row[k] for k in ('presented', 'zero', 'not_submitted', 'missing', 'invalid_clock'))
    if (row['attempted'] == 0 or row['attempted'] != row['admitted']
            or row['admitted'] != settled or row['presented'] + row['zero'] == 0
            or any(row[k] for k in ('saturated', 'duplicate_callbacks', 'histogram_overflow'))):
        raise ValueError('Incomplete or inconsistent presentation retirement activity')
    return row


def records(output, *, warmups, measurements):
    rows = [record(line) for line in output.splitlines() if line.startswith(PREFIX)]
    total = warmups + measurements
    if ([r['cycle'] for r in rows] != list(range(1, total + 1))
            or len({r['session'] for r in rows}) != total
            or len({r['window'] for r in rows}) != total):
        raise ValueError('Missing, reordered or reused presentation retirement identities')
    return dict(cycles=rows, scope='Active bounded presentation state dropped before closed-window checkpoints; no presentation timing or frame-delivery acceptance')
