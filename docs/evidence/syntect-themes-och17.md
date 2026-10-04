# Syntect embedded-theme provenance — OCH-17

2026-10-04, macOS arm64, dirty checkout based on `83eb87e`. This closes the
untraced theme-source/root-notice collection gap identified in the
[asset review](embedded-assets-och17.md). It does not certify final licensing
completeness or change the shipped themes, dependencies or rendering behavior.

## Exact source chain

The published `syntect 5.3.0` crate records revision
`e4670846ecf16d8832db6c43d531bec466214e27`. Its
[submodule map](https://github.com/trishume/syntect/blob/e4670846ecf16d8832db6c43d531bec466214e27/.gitmodules)
and Git tree identify these theme inputs:

| Theme family | Exact submodule revision | Retained root license |
| --- | --- | --- |
| InspiredGitHub | `18ddb271179e118cfc2dd83abf88b915b7328a25` | [Seth Lopez](https://github.com/sethlopez/InspiredGitHub.tmtheme/blob/18ddb271179e118cfc2dd83abf88b915b7328a25/LICENSE) |
| Solarized, dark and light | `bcd6234b4f5f96d3fd27db079268b5757053072a` | [Koen Lageveen](https://github.com/braver/Solarized/blob/bcd6234b4f5f96d3fd27db079268b5757053072a/LICENSE) |
| Base16 Eighties, Mocha, Ocean dark and Ocean light | `2703e93f559e212ef3895edd10d861a4383ce93d` | [Gadzhi Kharkharov](https://github.com/SublimeText/Spacegray/blob/2703e93f559e212ef3895edd10d861a4383ce93d/LICENSE) |

The original InspiredGitHub and Spacegray repository names redirect to the
canonical repositories linked above. Retrieval used the recorded commits, not
their default branches. All seven `.tmTheme` files and three license files match
the Git blob SHA-1 identities in those trees. The three license texts are retained
unchanged under `third_party/licenses/registry/`; the exact paths, SHA-256 values,
Git blob identities and source URLs are in
[`notice-sources.json`](../../third_party/notice-sources.json).

The registry archive matches Cargo.lock SHA-256
`656b45c05d95a5704399aeef6bd0ddec7b2b3531b7c9e900abbf7c4d2190c925`.
Its `assets/default.themedump` matches the installed bytes and upstream Git blob
`a5ed231ae6c1ab3f1311b033b710f663ad315dc7`; dump SHA-256 is
`8b57a2118224993360b6fc5fc2fa2e9872a827f00f9c57d43da08fa42c892399`.

## Comparison with the shipped dump

Loading the seven source files with Syntect's theme parser and comparing the
complete decoded `Theme` values first found three exact matches and four
differences. The differences were investigated rather than silently ignored:

- InspiredGitHub and both Solarized themes match exactly, including metadata,
  settings and every highlighting scope/style rule.
- The four Base16 themes have root `gutterSettings` values. The parser at the
  dump's [last-change revision](https://github.com/trishume/syntect/blob/dd4947cf69d52ae1b44d5162bdcf9122c1fb1576/src/highlighting/theme_load.rs)
  does not load that root dictionary. The current 5.3.0 parser does. Consequently,
  the dump's `gutter` and `gutter_foreground` fields are `None` while freshly
  parsed values are present. Applying only that historical omission makes all
  four complete theme values equal; names, authors, scopes and every other setting
  remain part of the comparison.

This is exact semantic correspondence under the recorded older parser behavior,
not a claim that an unmodified 5.3.0 generator reproduces the dump byte-for-byte.
The application continues using its existing dump. The comparison uses local
Syntect parsing/deserialization; it does not render a window.

Root licenses are collection evidence, not a blanket resolution of derivative
authorship. For example, the Base16 files identify Chris Kempson as author and
Spacegray's README links to Base16. Preserve that provenance and review applicable
upstream/derivative notice obligations before final distribution. The raw root
license texts have not been rewritten to substitute a guessed copyright owner.

## Checks and collection

The supplemental manifest now has 54 package attributions, including three theme
license files assigned to the exact Syntect package/manifest. Ten portable notice
collector tests pass, including retained SHA-256/Git blob checks. Fresh offline
locked collection gives:

| Root | Packages | Copied files | Supplemental packages | Packages without text |
| --- | ---: | ---: | ---: | ---: |
| `gpuio-native` | 513 | 865 | 50 | 28 |
| `gpuio-signal-backend` | 485 | 823 | 50 | 28 |
| Gallery `gpuio-counter-backend` | 486 | 824 | 51 | 28 |

All **2,512 copied hashes** pass independent comparison. Each inventory retains
`license_review_complete=false`. Commands follow the
[notice collection instructions](../distribution.md), using fresh outputs
`{native,signal,gallery}-syntect-macos-001` in ignored
`scratch/agents/root-20261003-release-notices/`.

Local evidence there includes `syntect-theme-fetches-001.json`,
`syntect-theme-provenance-001.json`, the comparison source and
`syntect-theme-compare-003.log`, `syntect-notice-tests-001.log` and
`syntect-notices-hashes-001.log`. Initial `compare-001` records the real mismatch;
`compare-002` isolates its fields. Scratch artifacts are not build dependencies.
The full release's remaining package notices, OCaml/runtime/system/asset review,
physical qualification, distribution and publication gates remain open.
