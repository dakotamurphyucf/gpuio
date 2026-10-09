"""Explicit macOS OS-memory audit of a collector-owned, checkpointed process.

Keep this outside responsiveness measurements. These tools may suspend/sample
the process. Their categories are OS accounting, not a complete Metal census.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import time


def footprint_summary(document, pid):
    if (document.get('unit') != 'byte' or type(document.get('bytes per unit')) is not int
            or document['bytes per unit'] != 1):
        raise ValueError('Unsupported footprint units')
    if document.get('errors') != [] or document.get('warnings') != []:
        raise ValueError('Footprint reported errors/warnings or omitted diagnostics')
    processes = document.get('processes', [])
    if len(processes) != 1 or type(processes[0].get('pid')) is not int or processes[0]['pid'] != pid:
        raise ValueError('Footprint did not identify the exact owned process')
    process = processes[0]
    value = process.get('footprint')
    if type(value) is not int or value <= 0:
        raise ValueError('Missing positive physical footprint')
    categories = process.get('categories')
    if not isinstance(categories, dict) or not categories:
        raise ValueError('Missing footprint categories')
    for name, values in categories.items():
        if not name or not isinstance(values, dict):
            raise ValueError('Invalid footprint category')
        for field in ('dirty', 'swapped', 'clean', 'reclaimable', 'wired', 'regions'):
            if type(values.get(field)) is not int or values[field] < 0:
                raise ValueError('Missing or invalid footprint category accounting')
    return dict(pid=pid, name=process.get('name'), footprint_bytes=value, categories=categories)


def require_released_surfaces(samples):
    """Require zero IOSurface bytes and no growth in empty mapping counts.

    The macOS15 CI runner reports fixed zero-byte mappings after close.
    Regions count address-space mappings, not resident/dirty/wired bytes.
    Retain those counts and reject growth; neither result is a complete census
    of GPU allocations or proof of native object retirement by itself.
    """
    if not samples:
        raise ValueError('No closed-window physical-memory samples')
    fields = ('dirty', 'swapped', 'clean', 'reclaimable', 'wired')
    baseline_regions = None
    for sample in samples:
        regions = 0
        for name, values in sample['physical_memory']['categories'].items():
            if 'iosurface' not in name.lower():
                continue
            if set(values) != {*fields, 'regions'}:
                raise ValueError('Unsupported IOSurface accounting fields')
            if any(type(values[key]) is not int or values[key] < 0 for key in (*fields, 'regions')):
                raise ValueError('Invalid IOSurface accounting values')
            if any(values[key] for key in fields):
                raise ValueError(f'IOSurface bytes remain after closing cycle {sample["cycle"]}')
            regions += values['regions']
        if baseline_regions is None:
            baseline_regions = regions
        elif regions > baseline_regions:
            raise ValueError(f'Empty IOSurface mappings grew after closing cycle {sample["cycle"]}')


def sample(pid, directory):
    """Caller owns pid and holds the lifecycle checkpoint until this returns.

    Both tools have six-second timeouts: no next window is admitted while the
    tools run, and their combined timeout fits the child's 20-second handshake.
    Raw stdout/stderr and footprint JSON survive failure as well as success.
    """
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    results = []
    try:
        for name, args in (
            ('footprint', ['/usr/bin/footprint', '-p', str(pid), '-j', str(directory / 'footprint.json')]),
            ('vmmap', ['/usr/bin/vmmap', '-summary', str(pid)]),
        ):
            result = dict(tool=name, command=args, returncode=None)
            results.append(result)
            with (directory / f'{name}.stdout').open('wb') as stdout, (directory / f'{name}.stderr').open('wb') as stderr:
                completed = subprocess.run(args, stdout=stdout, stderr=stderr, timeout=6)
            result['returncode'] = completed.returncode
            if completed.returncode != 0:
                raise RuntimeError(f'{name} failed with status {completed.returncode}')
        document = json.loads((directory / 'footprint.json').read_text())
        summary = footprint_summary(document, pid)
        vmmap = (directory / 'vmmap.stdout').read_text()
        matches = re.findall(r'^Process:\s+.+\[(\d+)\]\s*$', vmmap, re.MULTILINE)
        if matches != [str(pid)] or not re.search(r'^Physical footprint:\s+\S+', vmmap, re.MULTILINE):
            raise ValueError('vmmap did not identify the exact owned process and footprint')
        for name in ('footprint', 'vmmap'):
            if (directory / f'{name}.stderr').read_text().strip():
                raise ValueError(f'{name} reported diagnostic stderr; inspect retained artifacts')
        artifacts = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                     for p in directory.iterdir() if p.is_file()}
        return dict(**summary, elapsed_seconds=time.monotonic() - started,
                    artifacts=artifacts, directory=directory.name)
    finally:
        (directory / 'tools.json').write_text(json.dumps(results, indent=2) + '\n')
