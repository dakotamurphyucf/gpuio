#!/usr/bin/env python3
"""Check example ownership, links/fences and render its review checklist.

Existence/link checks do not establish that prose explains a component correctly.
Set review=reviewed only after the OCH-48 content review in coverage-guide.md.
"""
import argparse
import json
from functools import lru_cache
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
EXAMPLES = ROOT / 'examples'
ROLES = {
    'application-entry', 'component', 'support', 'test-support', 'benchmark',
    'historical-bootstrap', 'diagnostic', 'extension-author',
    'generated-registration',
}


def inventory():
    names = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z',
         '--', 'examples'], cwd=ROOT).decode().split('\0')
    return {name for name in names if Path(name).suffix in {'.ml', '.mli', '.rs'}}


def link(name):
    path = Path(name)
    return f'[{path.name}]({path.relative_to("examples").as_posix()})'


def check_fences(path):
    """Catch broken runnable blocks, without pretending to validate their code."""
    opening = None
    for number, line in enumerate(path.read_text().splitlines(), 1):
        match = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line)
        if not match:
            continue
        fence, info = match.groups()
        if opening:
            marker, length, _ = opening
            if fence[0] == marker and len(fence) >= length and not info.strip():
                opening = None
        else:
            if re.match(r'(sh|bash|shell)[./\\]', info.strip()):
                raise ValueError(f'{path}:{number}: shell command joined to fence label')
            opening = fence[0], len(fence), number
    if opening:
        raise ValueError(f'{path}:{opening[2]}: unclosed Markdown fence')


@lru_cache(maxsize=None)
def reachable_guides(directory):
    """Follow example prose links; the generated coverage table is not onboarding."""
    pending = [directory / 'README.md']
    seen = set()
    excluded = {EXAMPLES / 'coverage.md', EXAMPLES / 'coverage-guide.md'}
    while pending:
        path = pending.pop().resolve()
        if (path in seen or path in excluded or not path.is_relative_to(directory)
                or path.suffix != '.md' or not path.is_file()):
            continue
        seen.add(path)
        check_fences(path)
        # Repository guides use inline Markdown links. Ignore example code blocks.
        prose = re.sub(r'^```.*?^```[^\n]*$', '', path.read_text(),
                       flags=re.MULTILINE | re.DOTALL)
        for target in re.findall(r'\]\(([^)]+)\)', prose):
            target = unquote(target.strip('<>').split('#')[0])
            if not target or re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target):
                continue
            destination = (path.parent / target).resolve()
            if destination.is_dir():
                destination /= 'README.md'
            pending.append(destination)
    return seen


def audit():
    value = json.loads((EXAMPLES / 'coverage.json').read_text())
    if value['schema_version'] != 1:
        raise ValueError('Unsupported example documentation inventory')
    seen = set()
    lines = ['# Example walkthrough coverage', '',
             'Generated from [coverage.json](coverage.json). Read the '
             '[review guide](coverage-guide.md) before changing status.', '',
             '**Pending means incomplete review**, even where a README exists. '
             'A reviewed row is documentation coverage, not platform acceptance.', '']
    application = None
    reviewed = 0
    for row in value['components']:
        sources, docs = row['sources'], row['walkthroughs']
        if (not sources or row['role'] not in ROLES
                or row['review'] not in {'pending', 'reviewed'}):
            raise ValueError(f'Invalid component row: {row}')
        if len(set(sources)) != len(sources) or seen.intersection(sources):
            raise ValueError(f'Repeated source ownership: {sources}')
        seen.update(sources)
        for name in sources + docs:
            path = Path(name)
            if (path.is_absolute() or '..' in path.parts or path.parts[0] != 'examples'
                    or not (ROOT / path).is_file()):
                raise ValueError(f'Missing or invalid example path: {name}')
        if any(Path(doc).suffix != '.md' for doc in docs):
            raise ValueError(f'Walkthroughs must be Markdown: {docs}')
        if row['review'] == 'reviewed':
            if not docs:
                raise ValueError(f'Reviewed source has no walkthrough: {sources}')
            reviewed += 1
        app = Path(sources[0]).parts[1]
        if any(Path(name).parts[1] != app for name in sources):
            raise ValueError(f'Cross-application source group: {sources}')
        if row['review'] == 'reviewed':
            for directory in (EXAMPLES, EXAMPLES / app):
                for doc in docs:
                    if (ROOT / doc).resolve() not in reachable_guides(directory):
                        raise ValueError(f'{doc} is not reachable through prose links '
                                         f'from {directory.relative_to(ROOT)}/README.md')
        if app != application:
            application = app
            lines.extend([f'## {app}', '',
                          '| Source parts | Role | Walkthrough | Review |',
                          '| --- | --- | --- | --- |'])
        lines.append('| ' + ' | '.join([
            ', '.join(map(link, sources)), row['role'],
            ', '.join(map(link, docs)) or 'Missing', row['review']]) + ' |')
    actual = inventory()
    if actual != seen:
        raise ValueError(f'Source inventory mismatch: missing={sorted(actual-seen)}, '
                         f'obsolete={sorted(seen-actual)}')
    lines.extend(['', f'{len(seen)} source files in {len(value["components"])} groups; '
                  f'{reviewed} reviewed, {len(value["components"])-reviewed} pending.', ''])
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true',
                        help='Render the checklist after manually reviewing inventory edits')
    args = parser.parse_args()
    rendered = audit()
    output = EXAMPLES / 'coverage.md'
    if args.write:
        output.write_text(rendered)
    elif not output.exists() or output.read_text() != rendered:
        raise SystemExit('Example coverage table is stale; review inventory then use --write')
    print(rendered.splitlines()[-1])
    print('EXAMPLE_DOC_INVENTORY_OK: structural coverage only; pending work remains explicit')


if __name__ == '__main__':
    main()
