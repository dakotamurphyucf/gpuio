# Third-party asset notices

Individual third-party assets retain their own licenses. The repository's root
Apache-2.0 license does not replace those terms. This file supplements the notices
retained with vendored sources; it is not a completed inventory of every dependency
or an application-distribution notice bundle.

## Dragon clip art

`vendor/gpui/examples/svg/dragon.svg` contains **Dragon clip art.svg** by
**Ebaychatter0**, published on 2 October 2012 at
[Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Dragon_clip_art.svg).
The illustration is licensed under
[Creative Commons Attribution-ShareAlike 3.0 Unported](https://creativecommons.org/licenses/by-sa/3.0/)
([license text](https://creativecommons.org/licenses/by-sa/3.0/legalcode.en)).

The artwork is unchanged. The pinned upstream GPUI copy adds two XML comments
identifying the source and license. GPUIO retains that upstream copy. The SVG is
used by GPUI's vendored SVG example; this attribution does not suggest endorsement
of GPUIO by its author.

Provenance checked on 2026-10-08:

- Original: [download](https://upload.wikimedia.org/wikipedia/commons/d/d7/Dragon_clip_art.svg),
  SHA-1 `39fc8177d0ac34f759647e90a5c2647668691db9`, matching the Commons file record;
  SHA-256 `ddb134eb900db39f61d5514ad8ff58bde59d04a259781e4317e4e14e2b861bde`.
- Retained copy: SHA-256
  `a66278ec0c72d70152ab05fc9dd122ad094a262d96fa6f61fba6b12e128e0e47`.
  Removing only the two source/license comments makes its bytes identical to the
  original. No image or vendor-source change was made for this notice.
- Pinned GPUI source: Zed commit
  `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`, recorded in
  [sources.json](sources.json). The upstream source and license comments remain
  inside the SVG as well.

## Fira Code font in the retained Bonsai web example

`vendor/bonsai/examples/font_hosting/font.ttf` is **Fira Code Regular 6.002**.
Its metadata identifies Copyright 2014–2021 The Fira Code Project Authors and
the SIL Open Font License 1.1. The complete upstream copyright/license text is
retained unchanged in [fira-code.txt](licenses/fira-code.txt), fetched from the
[6.2 release source](https://github.com/tonsky/FiraCode/blob/6.2/LICENSE).
The font remains under that license; GPUIO's Apache-2.0 license does not replace it.

The vendored bytes exactly match `ttf/FiraCode-Regular.ttf` in the official
[6.2 release archive](https://github.com/tonsky/FiraCode/releases/tag/6.2).
Verification on 2026-10-08:

- Font SHA-256: `5992ab9640e2df491b2f609467b1de60e8bc39b2c28db184342a0592d98f6117`.
- `Fira_Code_v6.2.zip` SHA-256: `0949915ba8eb24d89fd93d10a7ff623f42830d7c5ffc3ecbf960e4ecad3e3e79`.
- Retained license SHA-256: `1d41e10031ab125302780a05ec4c91d218e47db0c7e37cf315cce5e608cdc25c`.

This unmodified font is retained in the source snapshot's unbuilt browser example;
it is not the application's system font or a newly added native dependency.

## Existing browser binding notices

The retained Bonsai browser bindings include Feather icons by Cole Bemis under
MIT, with their original [notice](../vendor/bonsai/bindings/feather_icon/dist/LICENSE-feather).
Dygraph and Lodash notices remain in
[`bindings/dygraph/dist`](../vendor/bonsai/bindings/dygraph/dist). These source
assets are included in the repository even though the native Dune subset does
not compile the browser bindings. Keep their existing notices with the sources.
