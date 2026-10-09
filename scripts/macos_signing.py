"""Developer ID signing for the reference apps' single-executable bundles.

No credential import, notarization submission or Gatekeeper approval is implied.
"""
import plistlib
import re
import subprocess


def validate_options(mode, identity=None, team_id=None):
    if mode not in ('unsigned', 'ad-hoc', 'developer-id'):
        raise ValueError('Unknown signing mode')
    if mode != 'developer-id':
        if identity is not None or team_id is not None:
            raise ValueError('Identity and team are only valid for developer-id signing')
        return
    if not isinstance(identity, str) or not re.fullmatch(r'[0-9A-Fa-f]{40}', identity):
        raise ValueError('Developer ID requires the certificate SHA-1 fingerprint (40 hex digits)')
    if not isinstance(team_id, str) or not re.fullmatch(r'[A-Z0-9]{10}', team_id):
        raise ValueError('Developer ID requires a ten-character team ID')


def requirement(identity, team_id, identifier):
    validate_options('developer-id', identity, team_id)
    if not isinstance(identifier, str) or not re.fullmatch(r'[A-Za-z0-9.-]+', identifier):
        raise ValueError('Invalid signing identifier')
    # This is a verification constraint (-R), not a replacement designated
    # requirement. Apple's Developer ID issuer/leaf OIDs come from TN3127.
    return ('anchor apple generic'
            ' and certificate 1[field.1.2.840.113635.100.6.2.6] exists'
            ' and certificate leaf[field.1.2.840.113635.100.6.1.13] exists'
            f' and certificate leaf[subject.OU] = "{team_id}"'
            f' and certificate leaf = H"{identity}"'
            f' and identifier "{identifier}"')


def run(*arguments):
    return subprocess.check_output(arguments, text=True, stderr=subprocess.STDOUT,
                                   timeout=60).strip()


def check_details(details, team_id, identifier):
    fields = {}
    for key in ('TeamIdentifier', 'Identifier', 'Timestamp'):
        values = re.findall(r'^' + key + r'=(.+)$', details, re.MULTILINE)
        if len(values) != 1 or values[0].strip().lower() in ('none', 'not set', ''):
            raise ValueError(f'Missing or ambiguous signature {key}')
        fields[key] = values[0]
    if fields['TeamIdentifier'] != team_id or fields['Identifier'] != identifier:
        raise ValueError('Signature team or application identifier mismatch')
    flags = re.findall(r'\bflags=0x([0-9a-fA-F]+)\b', details)
    if len(flags) != 1 or not int(flags[0], 16) & 0x10000:
        raise ValueError('Signature does not enable hardened runtime')
    return fields


def verify(bundle, identity, team_id, identifier):
    constraint = requirement(identity, team_id, identifier)
    run('/usr/bin/codesign', '--verify', '--strict', '--all-architectures',
        '--verbose=2', '-R', '=' + constraint, str(bundle))
    metadata = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
    name = metadata['CFBundleExecutable']
    if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9_-]+', name):
        raise ValueError('Invalid bundle executable name')
    executable = bundle / 'Contents/MacOS' / name
    arches = run('/usr/bin/lipo', '-archs', str(executable)).split()
    if not arches or len(set(arches)) != len(arches) or not set(arches) <= {'arm64', 'x86_64'}:
        raise ValueError('Unexpected signed executable architectures')
    return {arch: check_details(run('/usr/bin/codesign', '--display', '--verbose=4',
                                   '--architecture', arch, str(bundle)), team_id, identifier)
            for arch in arches}


def sign(bundle, identity, team_id, identifier):
    requirement(identity, team_id, identifier)  # Validate before modifying files.
    run('/usr/bin/codesign', '--force', '--sign', identity, '--options', 'runtime',
        '--timestamp', '--identifier', identifier, str(bundle))
    return verify(bundle, identity, team_id, identifier)
