#!/usr/bin/env python3
"""Check example source ownership and render its documentation review checklist.

Existence/link checks do not establish that prose explains a component correctly.
Set review=reviewed only after the OCH-48 content review in coverage-guide.md.
"""
import argparse
import json
from pathlib import Path
import subprocess

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
