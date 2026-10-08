#!/usr/bin/env python3
"""Finalize an already submitted Developer ID reference package; never upload it.

Fetch the existing submission's log, bind acceptance to the exact input archive,
staple a private extraction and verify the final archive after round-trip extraction.
This does not attest license review, quarantine transfer or clean-machine behavior.
"""
import argparse
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import uuid

import macos_signing
from package_macos_reference import digest
from test_macos_package_runtime import extract, inspect_package


def submission_id(value):
    if not isinstance(value, str):
        raise ValueError('Use a canonical notary submission UUID')
    parsed = str(uuid.UUID(value))
    if parsed != value.lower():
        raise ValueError('Use a canonical notary submission UUID')
    return parsed


def accepted_log(log, identifier, archive_hash):
    if (not isinstance(log, dict) or type(log.get('logFormatVersion')) is not int
            or log['logFormatVersion'] != 1
            or submission_id(log.get('jobId', '')) != identifier
            or log.get('status') != 'Accepted'
            or not isinstance(log.get('sha256'), str)
            or log.get('sha256', '').lower() != archive_hash):
        raise ValueError('Notary log must accept this submission and exact archive hash')
    issues = log.get('issues')
    if issues is None:
        issues = []
    if not isinstance(issues, list) or any(
            not isinstance(item, dict) or item.get('severity') != 'warning' for item in issues):
        raise ValueError('Unexpected or non-warning issues in accepted notary log')
    return issues


def run(*arguments):
    return subprocess.check_output(arguments, text=True, stderr=subprocess.STDOUT,
                                   timeout=120).strip()


def finalize(package, output, identifier, profile):
    identifier = submission_id(identifier)
    if not profile or any(ord(c) < 32 for c in profile):
        raise ValueError('Supply an existing Keychain profile name')
    if platform.system() != 'Darwin':
        raise ValueError('Notarization finalization requires macOS')
    original, archive = inspect_package(package)
    if original['signing'] != 'developer-id':
        raise ValueError('Only a Developer ID package can be finalized')
    if output.exists():
        raise ValueError('Choose a fresh output directory; existing evidence is preserved')
    if output.resolve().is_relative_to(package.resolve()):
        raise ValueError('Keep finalized output separate from the submission input')
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'submission_id': identifier,
              'submitted_archive_sha256': original['archive_sha256'],
              'qualification': 'ticket/signature checks only; no clean-machine or quarantine acceptance'}
    final_package = {**original, 'complete': False}
    try:
        # Freeze the submitted bytes; validation never trusts the live input .app.
        with tempfile.TemporaryDirectory(prefix='gpuio-notary-input-') as temporary:
            frozen = Path(temporary) / 'input'
            frozen.mkdir()
            shutil.copyfile(archive, frozen / original['archive'])
            (frozen / 'package.json').write_text(json.dumps(original))
            _, executable = extract(frozen, output)
            bundle = output / original['bundle']
            log_path = output / 'notary-log.json'
            run('/usr/bin/xcrun', 'notarytool', 'log', identifier,
                '--keychain-profile', profile, str(log_path))
            issues = accepted_log(json.loads(log_path.read_text()), identifier,
                                  original['archive_sha256'])
            report['warnings'] = issues
            report['notary_log_sha256'] = digest(log_path)
            report['staple'] = run('/usr/bin/xcrun', 'stapler', 'staple', str(bundle))
            report['validate'] = run('/usr/bin/xcrun', 'stapler', 'validate', str(bundle))
            report['signature'] = macos_signing.verify(
                bundle, original['signing_identity'], original['signing_team'],
                original['metadata']['CFBundleIdentifier'])
            if digest(executable) != original['packaged_executable_sha256']:
                raise ValueError('Stapling changed the signed executable bytes')
            final_archive = output / original['archive']
            run('/usr/bin/ditto', '-c', '-k', '--sequesterRsrc', '--keepParent',
                str(bundle), str(final_archive))
            final_package.update(archive_sha256=digest(final_archive),
                                 notarization={'submission_id': identifier,
                                               'submitted_archive_sha256': original['archive_sha256'],
                                               'log_sha256': report['notary_log_sha256'],
                                               'stapled': True})
            # Verify what will be transferred, including preserved ticket and
            # original metadata/notices/executable, before marking output complete.
            check = Path(temporary) / 'final'
            check.mkdir()
            shutil.copyfile(final_archive, check / final_package['archive'])
            (check / 'package.json').write_text(json.dumps({**final_package, 'complete': True}))
            restored = Path(temporary) / 'restored'
            extract(check, restored)
            report['round_trip_validate'] = run(
                '/usr/bin/xcrun', 'stapler', 'validate', str(restored / original['bundle']))
            report['final_archive_sha256'] = final_package['archive_sha256']
            report['complete'] = True
            final_package['complete'] = True
            final_package['qualification'] = (
                'notary acceptance bound to submitted archive; stapled final archive verified; '
                'no runtime, license-completeness, clean-machine or quarantine acceptance')
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (output / 'notarization.json').write_text(json.dumps(report, indent=2) + '\n')
        (output / 'package.json').write_text(json.dumps(final_package, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--submission-id', required=True)
    parser.add_argument('--keychain-profile', required=True)
    args = parser.parse_args()
    report = finalize(args.package.resolve(), args.output.resolve(),
                      args.submission_id, args.keychain_profile)
    print('NOTARIZATION_FINALIZED', report['final_archive_sha256'],
          'warnings=', len(report['warnings']))


if __name__ == '__main__':
    main()
