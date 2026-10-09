#!/usr/bin/env python3
"""Measure 100k logical rows with bounded application payloads and 64 columns.

Uses the same native histogram reader, process collector and predeclared draw/RSS
budgets as loaded lists. Source metadata remains O(logical rows). The deterministic
payload generator is not remote-I/O throughput or a boundary-pager benchmark.
"""
from pathlib import Path

from measure_list_history import ZERO_RESOURCES, fields, main, natural, read_interval, sexp


def validate(output, *, smoke, background, wall_clock=False):
    records = []
    for line in output.splitlines():
        if line.startswith('GPUIO_TABLE_PERF '):
            name, payload = line[len('GPUIO_TABLE_PERF '):].split(' ', 1)
            records.append((name, sexp(payload)))
    position = 0

    def take(name):
        nonlocal position
        if position >= len(records) or records[position][0] != name:
            raise ValueError(f'Expected table {name} at record {position}')
        value = records[position][1]
        position += 1
        return value

    rows, idle = (1030, 2) if smoke else (100000, 60)
    if take('config') != [str(rows), '64', '128', '4', str(smoke).lower(),
                          str(background).lower(), str(idle)]:
        raise ValueError('Table workload configuration mismatch')
    result = {}
    for phase in ('history', 'idle'):
        before_capture = 0
        if wall_clock:
            if take('wall-begin') != phase:
                raise ValueError('Wrong wall-clock phase')
        else:
            name, capture = take('begin')
            if name != phase:
                raise ValueError('Wrong table phase')
            before_capture = natural(capture)
        if phase == 'history' and take('interactions') != ['0', '0', str(rows // 2), '63']:
            raise ValueError('Selection/reveal evidence missing or wrong')
        interval = read_interval(take, phase, wall_clock=wall_clock, before_capture=before_capture)
        result[phase] = interval
        if phase == 'history':
            names = ('forward', 'backward', 'materialized', 'columns', 'peak_active_rows',
                     'peak_active_cells', 'peak_loaded_rows', 'page_loads', 'page_evictions', 'loaded_rows')
            values = take('history')
            if len(values) != len(names):
                raise ValueError('Wrong table coverage fields')
            coverage = dict(zip(names, map(natural, values)))
            if any(coverage[k] != rows for k in ('forward', 'backward', 'materialized')):
                raise ValueError('Incomplete table row coverage')
            if coverage['columns'] != 64:
                raise ValueError('Incomplete column coverage')
            if (not 0 < coverage['peak_active_rows'] <= 32
                    or coverage['peak_active_cells'] != 64 * coverage['peak_active_rows']
                    or not 0 < coverage['loaded_rows'] <= coverage['peak_loaded_rows'] <= 512):
                raise ValueError('Table active/cache budgets violated')
            pages = (rows + 127) // 128
            if coverage['page_loads'] < pages or coverage['page_evictions'] < pages - 4:
                raise ValueError('Paging/eviction coverage is missing')
            result['coverage'] = coverage
        elif ((not wall_clock and any(h['count'] for h in interval['histograms'].values()))
              or interval['elapsed_ns'] < idle * 1_000_000_000):
            raise ValueError('Table idle interval too short or contained native work')
    before, after, changes = take('idle-activation')
    if before not in ([], ['true'], ['false']) or after not in ([], ['true'], ['false']):
        raise ValueError('Invalid window activation observation')
    result['idle_activation'] = dict(before=before, after=after, changes=natural(changes),
                                     scope='Asynchronous window snapshots through final histogram retrieval; not OS visibility')
    cleanup = fields(take('cleanup'))
    if any(natural(cleanup[k]) for k in ZERO_RESOURCES):
        raise ValueError('Table resources remained after close')
    queue = fields(cleanup['native_command_queue'])
    if natural(queue['commands']) or natural(queue['bytes']):
        raise ValueError('Table native command queue not drained')
    result['cleanup'] = cleanup
    if natural(take('complete')) != rows or position != len(records):
        raise ValueError('Incomplete or duplicate table completion')
    return result


if __name__ == '__main__':
    main(validate_workload=validate,
         default_executable=Path('_build/default/examples/performance_table/main.exe'),
         success_marker='TABLE_HISTORY_MEASUREMENT_OK')
