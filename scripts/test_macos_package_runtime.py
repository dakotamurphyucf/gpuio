#!/usr/bin/env python3
"""Exercise extracted reference apps with development-directory access denied.

The parent harness may read the checkout; only the app receives the sandbox and
minimal environment. This is runtime isolation on an existing Mac, not a clean
machine, Gatekeeper, notarization or license-completeness assertion.
"""
import argparse
import faulthandler
import json
import os
from pathlib import Path, PurePosixPath
import platform
import plistlib
import signal
import stat
import subprocess
import tempfile
import zipfile

import macos_signing

from package_macos_reference import APPS, ROOT, check_metadata, digest


def deny_profile(paths):
    paths = sorted({str(Path(path).resolve()) for path in paths})
    if not paths or any(path == '/' or any(ord(c) < 32 for c in path) for path in paths):
        raise ValueError('Invalid runtime isolation path')
    return '(version 1)\n(allow default)\n' + ''.join(
        f'(deny file-read* file-write* (subpath {json.dumps(path, ensure_ascii=False)}))\n'
        for path in paths)


def inspect_package(package):
    report = json.loads((package / 'package.json').read_text())
    app = report['app']
    name, _, executable = APPS[app]
    if (report['complete'] is not True or report['bundle'] != name + '.app'
            or report['archive'] != name + '.zip'):
        raise ValueError('Incomplete or unexpected reference package')
    macos_signing.validate_options(report['signing'], report.get('signing_identity'),
                                   report.get('signing_team'))
    check_metadata(report['metadata'], app)
    archive = package / report['archive']
    if archive.is_symlink() or digest(archive) != report['archive_sha256']:
        raise ValueError('Package archive hash/type mismatch')
    # ditto preserves the actual macOS bundle metadata, after a separate path
    # check excludes archive entries that could escape our extraction directory.
    with zipfile.ZipFile(archive) as zipped:
        names = set()
        total = 0
        for info in zipped.infolist():
            path = PurePosixPath(info.filename)
            total += info.file_size
            if (path.is_absolute() or '..' in path.parts or '\\' in info.filename
                    or not path.parts or path.parts[0] not in (report['bundle'], '__MACOSX')
                    or str(path) in names
                    or stat.S_IFMT(info.external_attr >> 16) not in (0, stat.S_IFREG, stat.S_IFDIR)
                    or total > 1024**3):
                raise ValueError('Unsafe, duplicate or oversized archive entry')
            names.add(str(path))
        expected = f"{report['bundle']}/Contents/MacOS/{executable}"
        if expected not in names:
            raise ValueError('Archive has no expected application executable')
    return report, archive


def extract(package, destination):
    report, archive = inspect_package(package)
    subprocess.run(['/usr/bin/ditto', '-x', '-k', str(archive), str(destination)],
                   check=True, timeout=60)
    bundle = destination / report['bundle']
    executable = bundle / 'Contents/MacOS' / report['metadata']['CFBundleExecutable']
    if digest(executable) != report['packaged_executable_sha256']:
        raise ValueError('Extracted executable hash mismatch')
    metadata = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
    if metadata != report['metadata']:
        raise ValueError('Extracted metadata mismatch')
    for name, expected in report['notices'].items():
        relative = PurePosixPath(name)
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError('Invalid notice path')
        if digest(bundle / 'Contents/Resources/Notices' / name) != expected:
            raise ValueError('Extracted notice hash mismatch')
    if report['signing'] == 'ad-hoc':
        subprocess.run(['/usr/bin/codesign', '--verify', '--strict', str(bundle)],
                       check=True, timeout=30)
    elif report['signing'] == 'developer-id':
        macos_signing.verify(bundle, report['signing_identity'], report['signing_team'],
                             report['metadata']['CFBundleIdentifier'])
    return report, executable


def minimal_environment():
    # Preserve the real user's HOME; never repoint toolchains or switches.
    allowed = ('HOME', 'USER', 'LOGNAME', 'TMPDIR', 'LANG', 'LC_CTYPE')
    return {**{key: os.environ[key] for key in allowed if key in os.environ},
            'PATH': '/usr/bin:/bin:/usr/sbin:/sbin'}


def check_denial(prefix, environment, directory, paths):
    observations = []
    for path in paths:
        if not path.exists():
            observations.append({'path': str(path), 'exists': False})
            continue
        # Stat the directory itself: this probes every existing deny root, even
        # when no known regular file can be assumed inside a toolchain cache.
        child = subprocess.run([*prefix, '/usr/bin/stat', '-f', '%N', str(path)],
                               cwd=directory, env=environment, capture_output=True,
                               text=True, timeout=10)
        if child.returncode == 0 or 'Operation not permitted' not in child.stderr:
            raise RuntimeError(f'Directory denial was not enforced: {path}')
        observations.append({'path': str(path), 'exists': True, 'denied': True})
    return observations


def exercise(app, child, output, workspace, board):
    # CoreFoundation can terminate an invalid native call with SIGTRAP. The
    # default faulthandler set omits that signal, so preserve its Python stack
    # before chaining to the original fatal behavior (never turn it into a pass).
    faulthandler.enable()
    faulthandler.register(signal.SIGTRAP, all_threads=True, chain=True)
    from test_agent_chat import Mac
    mac = None
    try:
        if app == 'gallery':
            from test_gallery import TITLE, exercise_assets
            from test_macos_text_selection import exercise as selections
            from test_canvas import screenshot
            mac = Mac(child.pid, child)
            exercise_assets(mac, output)
            mac.press(TITLE, 'Images & icons')
            dark = mac.find(TITLE, 'Dark', 'AXButton')
            current, alternate = ('Dark', 'Light') if dark else ('Light', 'Dark')
            if dark:
                mac.release(dark)
            mac.press(TITLE, current)
            mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
            screenshot(mac, output / f'gallery-assets-{alternate.lower()}.png', title=TITLE)
            mac.press(TITLE, alternate)
            observations = selections(mac, board, source_only=False)
            mac.close(TITLE)
            return {'checks': ['embedded-images', 'native-actions', 'source-editor-selections'],
                    'selections': observations}
        if app == 'signal_studio':
            from test_signal_studio import Studio
            mac = Studio(child, output / 'application.log')
            mac.exercise(output, bundled=True)
            assert 'Run completion alerts are enabled.' not in mac.log_path.read_text()
            return {'checks': ['extension', 'canvas', 'charts', 'inspector', 'stream',
                               'responsive-state', 'remount', 'ordinary-close'],
                    'notification_authorization': mac.notification_authorization,
                    'notification_opt_in': False}
        from test_agent_chat import exercise as chat
        mac = Mac(child.pid, child)
        attachment = workspace / 'native-attachment.txt'
        chat(mac, attachment)
        return {'checks': ['search', 'send', 'draft-retention', 'error-retry', 'tabs',
                           'windows', 'native-picker-eio-attachment', 'theme', 'close-policy']}
    finally:
        if mac:
            mac.release(mac.app)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != 'Darwin':
        parser.error('Requires a real macOS desktop')
    from test_agent_chat import Mac
    from mac_clipboard import preserved_clipboard
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(),
              'qualification': 'existing Mac with denied development directories; not clean-machine or release approval'}
    # A bounded test owns one foreground child. SIGTERM and timeout also take the
    # cleanup path and the existing clipboard preservation context.
    def interrupted(signum, _frame):
        raise RuntimeError(f'Package runtime test interrupted by signal {signum}')
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    try:
        with tempfile.TemporaryDirectory(prefix='gpuio-extracted-') as directory:
            workspace = Path(directory).resolve()
            package, executable = extract(args.package.resolve(), workspace)
            denied = [ROOT.resolve(), Path('/opt/homebrew'), Path('/usr/local'),
                      Path.home() / '.cargo', Path.home() / '.rustup', Path.home() / '.opam']
            if any(workspace.is_relative_to(path.resolve()) for path in denied):
                raise ValueError('Temporary extraction overlaps a denied root')
            profile = deny_profile(denied)
            (output / 'runtime.sb').write_text(profile)
            prefix = ['/usr/bin/sandbox-exec', '-p', profile]
            environment = minimal_environment()
            report.update(app=package['app'], archive_sha256=package['archive_sha256'],
                          executable_sha256=digest(executable),
                          packaging_revision=package['revision'],
                          packaging_revision_scope=package['revision_scope'],
                          denied_paths=check_denial(prefix, environment, workspace, denied),
                          environment_keys=sorted(environment), cwd=str(workspace))
            observed = subprocess.check_output([*prefix, str(executable), '--print-info-plist'],
                                               cwd=workspace, env=environment, timeout=30)
            if plistlib.loads(observed) != package['metadata']:
                raise ValueError('Isolated application metadata differs')
            arguments = []
            if package['app'] == 'agent_chat':
                arguments = ['--native-test', '--directory', str(workspace)]
                (workspace / 'native-attachment.txt').write_text(
                    'Native attachment λ\nRead through Eio and rendered as a document.\n')
            elif package['app'] == 'signal_studio':
                arguments = ['--exit-on-close']
            with preserved_clipboard() as board, (output / 'application.log').open('w') as log:
                child = subprocess.Popen([*prefix, str(executable), *arguments], cwd=workspace,
                                         env=environment, stdout=log, stderr=subprocess.STDOUT)
                try:
                    signal.alarm(180)
                    report.update(exercise(package['app'], child, output, workspace, board))
                    if child.wait(timeout=15):
                        raise RuntimeError('Packaged app exited unsuccessfully')
                    if package['app'] == 'agent_chat' and 'GPUIO_AGENT_CHAT_NATIVE_APP_RETURNED' not in (output / 'application.log').read_text():
                        raise RuntimeError('Missing chat App.run return marker')
                finally:
                    signal.alarm(0)
                    if child.poll() is None:
                        child.terminate()
                        try:
                            child.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.wait()
            report['clipboard_restored'] = True
            report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    print(f'GPUIO_PACKAGE_RUNTIME_OK: {report["app"]}, extracted archive, enforced path denial, native interactions, shutdown')


if __name__ == '__main__':
    main()
