#!/usr/bin/env python3
"""Report CI disk space; optionally discard only this job's restored Rust outputs."""
import argparse
import json
import os
from pathlib import Path
import shutil


def prepare(root, *, discard_restored_target, environment):
    root = root.resolve()
    before = shutil.disk_usage(root).free
    removed = False
    if discard_restored_target:
        workspace = environment.get('GITHUB_WORKSPACE')
        if (environment.get('GITHUB_ACTIONS') != 'true' or not workspace
                or Path(workspace).resolve() != root):
            raise RuntimeError('Discard is restricted to the current GitHub Actions workspace')
        if not (root / 'Cargo.lock').is_file() or not (root / 'dune-project').is_file():
            raise RuntimeError('Expected GPUIO workspace markers are absent')
        target = root / 'target'
        if target.is_symlink() or (target.exists() and not target.is_dir()):
            raise RuntimeError('Refusing a symlink or non-directory Rust target')
        if target.exists():
            shutil.rmtree(target)
            removed = True
    return dict(free_bytes_before=before, free_bytes_after=shutil.disk_usage(root).free,
                discarded_restored_target=removed)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--discard-restored-target', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    print(json.dumps(prepare(root, discard_restored_target=args.discard_restored_target,
                             environment=os.environ)), flush=True)


if __name__ == '__main__':
    main()
