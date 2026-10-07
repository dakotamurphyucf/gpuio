#!/usr/bin/env python3
"""Verify the pinned AccessKit consumer and its exact text-scope adaptation."""
import argparse
from pathlib import Path
import tempfile

from verify_accesskit_macos import verify


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True,
                        help='accesskit_consumer 0.38.0 registry archive')
    args = parser.parse_args()
    vendor = Path(__file__).resolve().parent.parent / 'vendor/accesskit-consumer'
    with tempfile.TemporaryDirectory(prefix='gpuio-accesskit-consumer-') as directory:
        verify(args.archive.resolve(), vendor, Path(directory))


if __name__ == '__main__':
    main()
