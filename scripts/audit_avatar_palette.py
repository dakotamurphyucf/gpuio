#!/usr/bin/env python3
"""Reproduce and check GPUIO's fixed avatar palette, not rendered accessibility.

L/C values and Oklab matrices come from the pinned, licensed source snapshots
component-avatar-avatar.rs.txt and component-theme-color.rs.txt. GPUIO fixes
8-bit sRGB via binary64 conversion, clipping and floor(channel*255+0.5).
"""
import argparse
import math
from pathlib import Path
import re


def rgb(lightness, chroma, hue):
    angle = math.radians(hue)
    a, b = chroma * math.cos(angle), chroma * math.sin(angle)
    l = (lightness + .3963377774 * a + .2158037573 * b) ** 3
    m = (lightness - .1055613458 * a - .0638541728 * b) ** 3
    s = (lightness - .0894841775 * a - 1.2914855480 * b) ** 3
    linear = (4.0767416621*l - 3.3077115913*m + .2309699292*s,
              -1.2684380046*l + 2.6097574011*m - .3413193965*s,
              -.0041960863*l - .7034186147*m + 1.7076147010*s)
    def byte(channel):
        encoded = 12.92 * channel if channel <= .0031308 else 1.055 * channel ** (1/2.4) - .055
        return math.floor(min(1, max(0, encoded)) * 255 + .5)
    red, green, blue = map(byte, linear)
    return red << 16 | green << 8 | blue


def palette(dark):
    values = ((.30,.05), (.82,.11), (.36,.06)) if dark else ((.97,.032), (.50,.145), (.89,.05))
    return [tuple(rgb(lightness, chroma, hue) for lightness, chroma in values)
            for hue in range(0, 360, 30)]


def luminance(rgb):
    def channel(shift):
        value = ((rgb >> shift) & 255) / 255
        return value / 12.92 if value <= .04045 else ((value + .055) / 1.055) ** 2.4
    return .2126 * channel(16) + .7152 * channel(8) + .0722 * channel(0)


def contrast(first, second):
    first, second = luminance(first), luminance(second)
    return (max(first, second) + .05) / (min(first, second) + .05)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--emit-ocaml', action='store_true')
    args = parser.parse_args()
    source = (Path(__file__).resolve().parent.parent / 'lib/core/avatar.ml').read_text()
    minimum = 21.
    for name, dark in [('light', False), ('dark', True)]:
        rows = palette(dark)
        minimum = min(minimum, *(contrast(bg, fg) for bg, fg, _ in rows))
        assert all(contrast(bg, fg) >= 4.5 for bg, fg, _ in rows)
        if args.emit_ocaml:
            print(f'  let {name} = [|')
            for bg, fg, border in rows:
                print(f'    0x{bg:06x}, 0x{fg:06x}, 0x{border:06x};')
            print('  |]\n')
        else:
            match = re.search(r'let '+name+r'\s*=\s*\[\|(.*?)\|\]', source, re.S)
            assert match, f'missing {name} palette'
            actual = [int(value, 16) for value in re.findall(r'0x[0-9a-fA-F]{6}\b', match.group(1))]
            assert actual == [value for row in rows for value in row], f'{name} palette changed'
    if not args.emit_ocaml:
        print(f'AVATAR_PALETTE_OK: 24 opaque RGB pairs; minimum contrast {minimum:.6f}:1; fixed bytes match pinned-source derivation')


if __name__ == '__main__':
    main()
