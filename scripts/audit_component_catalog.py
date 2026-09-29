#!/usr/bin/env python3
"""Verify pinned catalog inputs and their structural surface inventory.

This is an inventory check, not behavior or release acceptance. It intentionally
includes private root modules so helper/facade families cannot silently vanish.
Use --write only after reviewing an intentional upstream pin change.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / 'docs/catalog'


def interface(source, name):
    match = re.search(r'^export interface ' + re.escape(name) + r'(?: extends \w+)? \{\n(.*?)^\}',
                      source, re.M | re.S)
    if not match:
        raise ValueError(f'Missing interface {name}')
    return match.group(1)


def inventory():
    sources = CATALOG / 'sources'
    manifest = json.loads((sources / 'manifest.json').read_text())
    seen = set()
    for entry in manifest:
        name = entry['snapshot']
        if Path(name).name != name or name in seen:
            raise ValueError(f'Invalid or duplicate snapshot: {name}')
        seen.add(name)
        content = (sources / name).read_bytes()
        actual = hashlib.sha256(content).hexdigest()
        if actual != entry['sha256']:
            raise ValueError(f'Snapshot checksum mismatch: {name}')
        if not re.fullmatch(r'[0-9a-f]{40}', entry['revision']):
            raise ValueError(f'Unpinned revision: {name}')
    declared = {p.name for p in sources.iterdir() if p.is_file()} - {'manifest.json'}
    if declared != seen:
        raise ValueError(f'Manifest does not cover snapshots: {declared ^ seen}')
    result = {'schema_version': 1, 'status': 'source inventory; not implementation acceptance'}
    for layer in ['base', 'component']:
        source = (sources / f'{layer}-lib.rs.txt').read_text()
        result[f'gpui_{layer}_root_modules'] = sorted(set(re.findall(
            r'^(?:pub(?:\(crate\))? )?mod (\w+)', source, re.M)))
    host = (sources / 'gpuix-host.ts.txt').read_text()
    result['gpuix_style_fields'] = re.findall(r'^  (\w+)\??:', interface(host, 'StyleDesc'), re.M)
    result['gpuix_props_events'] = re.findall(r'^  (on\w+)\??:', interface(host, 'Props'), re.M)
    element_type = re.search(r'^export type ElementType =\n((?:  \| "[^\n]+"\n)+)', host, re.M)
    if not element_type:
        raise ValueError('Missing ElementType union')
    result['gpuix_intrinsic_elements'] = re.findall(r'"([^\"]+)"', element_type.group(1))
    index = (sources / 'gpuix-index.ts.txt').read_text()
    result['gpuix_export_modules'] = sorted(set(re.findall(r'from "([^"]+)"', index)))
    for key, values in result.items():
        if isinstance(values, list) and (not values or len(set(values)) != len(values)):
            raise ValueError(f'Empty or repeated inventory: {key}')
    return result


def audit_values_and_events(actual):
    manifest = json.loads((CATALOG / 'sources/manifest.json').read_text())
    pin = next(entry['revision'] for entry in manifest
               if entry['snapshot'] == 'gpuix-host.ts.txt')
    host = (CATALOG / 'sources/gpuix-host.ts.txt').read_text()
    style = (ROOT / 'lib/core/style.mli').read_text()
    values = json.loads((CATALOG / 'gpuix-values.json').read_text())
    events = json.loads((CATALOG / 'gpuix-events.json').read_text())
    for ledger in [values, events]:
        if ledger['revision'] != pin or ledger['schema_version'] != 1:
            raise ValueError('Value/event ledger must use the pinned GPUIX revision and schema')
    cursor = re.search(r'^export type CursorValue =\n((?:  \| "[^\n]+"\n)+)', host, re.M)
    overflow = re.search(r'^  textOverflow\?: ([^\n]+)', interface(host, 'StyleDesc'), re.M)
    gradient = re.search(r'^  colorSpace\?: ([^\n]+)', interface(host, 'LinearGradientBackground'), re.M)
    if not cursor or not overflow or not gradient:
        raise ValueError('Missing reviewed style value declarations')
    expected = {'cursor': ('Style', 'Cursor', re.findall(r'"([^"]+)"', cursor.group(1))),
                'textOverflow': ('Style', 'Text_overflow', re.findall(r'"([^"]+)"', overflow.group(1))),
                'background.colorSpace': ('Background', 'Color_space', re.findall(r'"([^"]+)"', gradient.group(1)))}
    for field, module in [('gridColumnMin', 'Grid_minimum'),
                          ('gridRowMin', 'Grid_minimum'),
                          ('whiteSpace', 'White_space'),
                          ('textDecoration', 'Text_decoration')]:
        declaration = re.search(r'^  ' + field + r'\?: ([^\n]+)',
                                interface(host, 'StyleDesc'), re.M)
        if not declaration:
            raise ValueError(f'Missing reviewed style declaration: {field}')
        expected[field] = ('Style', module, re.findall(r'"([^"]+)"', declaration.group(1)))
    interfaces = {'Style': style, 'Background': (ROOT / 'lib/core/background.mli').read_text()}
    if {row['source_field'] for row in values['rows']} != set(expected):
        raise ValueError('Value ledger must explicitly cover the reviewed fields only')
    for field, (owner, module, source_values) in expected.items():
        rows = [row for row in values['rows'] if row['source_field'] == field]
        mapped = [value for row in rows for value in row['source_values']]
        if len(mapped) != len(set(mapped)) or set(mapped) != set(source_values):
            raise ValueError(f'Style values missing or duplicated: {field}')
        declaration = re.search(r'^module ' + module + r' : sig\n(.*?)^end', interfaces[owner], re.M | re.S)
        if not declaration:
            raise ValueError(f'Missing style value module: {module}')
        constructors = set(re.findall(r'^    \| (\w+)', declaration.group(1), re.M))
        for row in rows:
            if (not row['source_values'] or row['public_type'] != f'{owner}.{module}'
                    or row['public_value'] not in constructors):
                raise ValueError(f'Invalid style value mapping: {row}')
    references = list(values['evidence'])
    names = [row['source_event'] for row in events['rows']]
    if len(names) != len(set(names)) or set(names) != set(actual['gpuix_props_events']):
        raise ValueError('Event ledger must cover each pinned event exactly once')
    for row in events['rows']:
        if (row['status'] not in {'mapped', 'mapped_with_difference', 'partial', 'gap'}
                or row['release_scope'] != 'v1'
                or not all(row[key] for key in ['owner', 'contract', 'remaining', 'platform'])
                or not row['public_interfaces']
                or not row['upstream'].startswith(f'https://github.com/remorses/gpuix/blob/{pin}/')):
            raise ValueError(f'Incomplete event audit: {row["source_event"]}')
        references.extend([*row['public_interfaces'], row['evidence']])
    for reference in references:
        if not (ROOT / reference).is_file():
            raise ValueError(f'Missing value/event reference: {reference}')
    print(f'Value mapping: {len(expected)} reviewed value sets; event contracts: {len(names)}; remaining gaps are explicit.')


def audit_native_values():
    """Check reviewed native branches; this deliberately does not infer CSS semantics."""
    ledger = json.loads((CATALOG / 'gpuix-native-values.json').read_text())
    manifest = {entry['snapshot']: entry for entry in
                json.loads((CATALOG / 'sources/manifest.json').read_text())}
    native = manifest['gpuix-renderer.rs.txt']
    helper = manifest['gpuix-gpui-styled.rs.txt']
    if (ledger['schema_version'] != 1 or ledger['revision'] != native['revision']
            or ledger['helper_revision'] != helper['revision']
            or helper['submodule_of']['revision'] != native['revision']
            or helper['submodule_of']['path'] != 'zed'
            or ledger['source'] != 'sources/gpuix-renderer.rs.txt'
            or ledger['helper_source'] != 'sources/gpuix-gpui-styled.rs.txt'):
        raise ValueError('Native value ledger must follow the pinned renderer and GPUI submodule')
    source = (CATALOG / ledger['source']).read_text()
    apply_styles = source.split('pub(crate) fn apply_styles<', 1)[1].split('\n    el\n}', 1)[0]
    inherited = source.split('fn descend(mut self, style: Option<&StyleDesc>)', 1)[1]
    inherited = inherited.split('\n        self\n', 1)[0]
    reviewed = {
        'display': ('display', 'Display', 'Display'),
        'visibility': ('visibility', 'Visibility', 'Visibility'),
        'overflow': ('overflow', 'Overflow', 'Overflow'),
        'overflowX': ('overflow_x', 'Overflow_x', 'Overflow'),
        'overflowY': ('overflow_y', 'Overflow_y', 'Overflow'),
        'flexDirection': ('flex_direction', 'Direction', 'Direction'),
        'flexWrap': ('flex_wrap', 'Wrap', 'Wrap'),
        'alignItems': ('align_items', 'Align_items', 'Align'),
        'alignSelf': ('align_self', 'Align_self', 'Align'),
        'alignContent': ('align_content', 'Align_content', 'Distribution'),
        'justifyContent': ('justify_content', 'Justify_content', 'Distribution'),
        'position': ('position', 'Position', 'Position'),
        'textAlign': ('text_align', 'Text_align', 'Text_align'),
        'userSelect': ('user_select', 'User_select', 'bool'),
    }
    names = [field['source_field'] for field in ledger['fields']]
    if len(names) != len(set(names)) or set(names) != set(reviewed):
        raise ValueError('Native value ledger must cover the reviewed fields exactly once')
    interface_source = (ROOT / 'lib/core/style.mli').read_text()
    properties = interface_source.split('module Property : sig', 1)[1].split('module Name : sig', 1)[0]
    for field in ledger['fields']:
        name = field['source_field']
        native_field, prop, module = reviewed[name]
        if name.startswith('overflow'):
            # Both the Styled hidden branch and the host's scroll branch resolve
            # axis longhands before the shorthand. Other strings do not apply.
            expected = sorted(set(re.findall(r'resolved_[xy] == Some\("([^"]+)"\)', source)))
        elif name == 'visibility':
            expected = re.findall(r'if style\.visibility\.as_deref\(\) == Some\("([^"]+)"\)',
                                  apply_styles)
        elif name == 'flexDirection':
            expected = re.findall(r'if style\.flex_direction\.as_deref\(\) == Some\("([^"]+)"\)',
                                  apply_styles)
        else:
            scope = inherited if name == 'userSelect' else apply_styles
            branch = scope.split(f'match style.{native_field}.as_deref() {{', 1)[1]
            indent = 8 if name == 'userSelect' else 4
            branch = re.split(r'^' + ' ' * indent + r'\}', branch, maxsplit=1, flags=re.M)[0]
            expected = re.findall(r'Some\("([^"]+)"\)', branch)
        mapped = [value for row in field['rows'] for value in row['source_values']]
        if not expected or len(mapped) != len(set(mapped)) or set(mapped) != set(expected):
            raise ValueError(f'Native keyword mapping missing or repeated: {name}')
        public_type = 'bool' if module == 'bool' else f'Style.{module}'
        property_type = 'bool' if module == 'bool' else f'{module}.t'
        if (field['public_property'] != prop or field['public_type'] != public_type
                or not field['note'] or not re.search(
                    r'^    \| ' + prop + r' of ' + re.escape(property_type) + r'$', properties, re.M)):
            raise ValueError(f'Invalid native value public property: {name}')
        constructors = set()
        if module != 'bool':
            declaration = re.search(r'^module ' + module + r' : sig\n(.*?)^end',
                                    interface_source, re.M | re.S)
            if not declaration:
                raise ValueError(f'Missing public style module: {module}')
            constructors = set(re.findall(r'^    \| (\w+)', declaration.group(1), re.M))
        for row in field['rows']:
            target = row['target']
            kind = target['kind']
            valid = (kind == 'constructor' and target.get('value') in constructors
                     or kind == 'boolean' and module == 'bool' and type(target.get('value')) is bool
                     or kind == 'unset' and name == 'alignContent'
                     and row['source_values'] == ['normal'] and set(target) == {'kind'})
            if not row['source_values'] or not valid:
                raise ValueError(f'Invalid native value target: {row}')
    for reference in ledger['evidence']:
        if not (ROOT / reference).is_file():
            raise ValueError(f'Missing native value evidence: {reference}')
    print(f'Native keyword mapping: {len(names)} reviewed fields; aliases and reset limits remain explicit.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true', help='replace structural inventory after pin review')
    args = parser.parse_args()
    actual = inventory()
    path = CATALOG / 'inventory.json'
    if args.write:
        path.write_text(json.dumps(actual, indent=2) + '\n')
    elif json.loads(path.read_text()) != actual:
        raise ValueError('Catalog inventory differs from pinned sources; review before --write')
    audit_values_and_events(actual)
    audit_native_values()
    styles = json.loads((CATALOG / 'gpuix-styles.json').read_text())['rows']
    names = [row['source_field'] for row in styles]
    if len(names) != len(set(names)) or set(names) != set(actual['gpuix_style_fields']):
        raise ValueError('Style mapping must cover each pinned field exactly once')
    source = (ROOT / 'lib/core/style.mli').read_text()
    properties = source.split('module Property : sig', 1)[1].split('module Name : sig', 1)[0]
    known = set(re.findall(r'^    \| (\w+)', properties, re.M))
    for row in styles:
        api = row['public_api']
        if api.startswith('Style.Property.'):
            if api.removeprefix('Style.Property.') not in known:
                raise ValueError(f'Style API no longer exists: {api}')
        elif api not in ('Style.with_state Hovered', 'Style.with_state Pressed'):
            raise ValueError(f'Unknown style mapping: {api}')
        for reference in [row['interface'], *row['evidence']]:
            if not (ROOT / reference).is_file():
                raise ValueError(f'Missing style reference: {reference}')
    families = json.loads((CATALOG / 'families.json').read_text())['families']
    expected_modules = {f'{layer}/{name}' for layer in ('base', 'component')
                        for name in actual[f'gpui_{layer}_root_modules']}
    mapped = [name for row in families for name in row['upstream_modules']]
    if len(mapped) != len(set(mapped)) or set(mapped) != expected_modules:
        raise ValueError('Family ownership must cover every pinned module exactly once')
    ids = [row['id'] for row in families]
    if len(ids) != len(set(ids)):
        raise ValueError('Duplicate family identity')
    for row in families:
        if not row['contract'] or not row['owner'] or not row['audit_status']:
            raise ValueError(f'Incomplete family metadata: {row["id"]}')
        for reference in [row['public_interface'], row['example'], *row['evidence']]:
            if not (ROOT / reference).exists():
                raise ValueError(f'Missing family reference: {reference}')
    # These accepted OCH-23..28 additions do not map to Longbridge root modules.
    # Motion is already in families.json; OCH-29 consumer/release gates are tracked
    # in the milestone evidence, rather than inventing another widget family.
    additions = json.loads((CATALOG / 'expanded-v1.json').read_text())['capabilities']
    expected_additions = {'canvas', 'native-extensions', 'container-rules',
                          'desktop-integration', 'os-notifications'}
    addition_ids = [row['id'] for row in additions]
    if len(addition_ids) != len(set(addition_ids)) or set(addition_ids) != expected_additions:
        raise ValueError('Accepted expanded-v1 capabilities are missing or duplicated')
    for row in additions:
        if row['scope'] != 'v1' or not row['contract'] or not row['owner'] or not row['audit_status']:
            raise ValueError(f'Incomplete expanded-v1 metadata: {row["id"]}')
        for reference in [row['public_interface'], row['example'], *row['evidence']]:
            if not (ROOT / reference).exists():
                raise ValueError(f'Missing expanded-v1 reference: {reference}')
    print('GPUIO_CATALOG_SOURCES_OK:', ', '.join(
        f'{key}={len(value)}' for key, value in actual.items() if isinstance(value, list)))
    print('Structural source coverage only; implementation, values and native behavior require the release ledger.')
    print(f'Family ownership: {len(mapped)} module entries -> {len(families)} families; detailed review status remains explicit.')
    print(f'Accepted additions outside that module map: {len(additions)}; release acceptance remains separate.')


if __name__ == '__main__':
    main()
