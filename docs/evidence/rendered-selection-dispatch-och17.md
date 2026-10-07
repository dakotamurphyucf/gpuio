# Rendered-document selection actions — OCH-17

Local macOS arm64 implementation after `8d37b39e`, 2026-10-07. Native Document
selection actions now use the validated preparation path and existing selection
owners. The rebuilt gallery passes actual macOS AX range mutation and native Copy.
This is scoped action evidence, not complete accessibility or release acceptance.

## Behavior and ownership

The Document listener accepts `SetTextSelection` only for selectable prepared
content. A weak entity, current window selection registration/scope and the
previously implemented projection/frame/interaction/host checks gate mutation.
The existing staged native-owner operation validates all owners before changing
any of them. Failed admission/application leaves other selections and focus alone.

After success, the window controller removes old drag geometry, detaches the
target's stale snapshot/Copy cache and preserves its new local selection. It runs
other participant clear callbacks outside entity leases. Superseded queued
snapshots are ignored, and a queued empty snapshot cannot erase a newer explicit
caret. Later explicit clearing still works. Native Copy reads the same owners;
there is no new OCaml protocol or synchronous OCaml callback.

The action focuses the native document and cancels link/control traversal. It
realizes the logical head's virtual block and uses prepaint's actual shaped caret
for reveal, including within blocks taller than the viewport. Atomic wrappers
have corresponding actual-bound edge handling. Structural separators reveal the
preceding owner edge without altering logical selection or Copy. Unlaid-out text
does not acquire fabricated rectangles.

## Native and build checks

The focused group passes nine tests. Three additions exercise platform action
routing and shared-window Copy, rejected malformed/guarded requests, Unicode
partial/backward ranges, native focus, caret retention, explicit clear, a queued
clear followed by a new caret, 120 offscreen paragraphs and a 120-line code block.
The reveal assertion checks the last selected glyph inside the viewport using the
actual display scale. A structural separator has no glyph rectangle; AccessKit
bounds are physical pixels, unlike GPUI's logical layout coordinates. Earlier
assertions confused those two boundaries; corrected tests retain the full logical
selection through its terminal separator.

All **1,162 native tests pass**, with two isolated Linux D-Bus fixtures ignored on
macOS. Strict workspace Clippy, Rust formatting, full Rust workspace tests and
actual macOS editor/document tests pass. The gallery rebuild also passes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
```

Scoped desktop checks took 57.452 seconds, workspace checks 136.765 seconds, and
the gallery rebuild 130.153 seconds. Full Dune suites were not repeated in this
cohort; their last complete result is at the earlier reader checkpoint.

The changed Base fork reconstructs exactly from its pinned source archive:
**243 files**, excluding Cargo.lock. Its patch SHA-256 is
`f05f6f523e85f82ba6d49c7b930bd04e13110e3d3dce61fec858f0ed83e56d3f`.
GPUI is unchanged from the preceding authorization checkpoint.

## Actual macOS probe

The successful experiment was promoted into the maintained regression:

```sh
python3 scripts/test_macos_text_selection.py --rendered-only \
  --output scratch/rendered-selection-check
```

The default mode now covers rendered documents before the existing source/editor
checks. `--source-only` retains its existing scope; the two narrow modes are
mutually exclusive. This checkpoint directly ran the new `--rendered-only` mode.

The OS setter, selected-text/range readers and native Command-C agree exactly:

| Selected text | UTF-16 location | UTF-16 length |
| --- | ---: | ---: |
| A place for ideas | 33 | 17 |
| 世界 | 91 | 2 |
| 👨‍👩‍👧‍👦 | 96 | 11 |
| Explore, inside the final code block | 392 | 7 |

A subsequent caret remains at `[392, 0]` with empty selected text. The whole-document
reading/Copy check also retains the existing single terminal logical separator
contract. Both the initial probe and maintained regression close their owned app
normally with exit 0 and restore/verify captured clipboard representations.
VoiceOver and system settings were untouched. Tested executable SHA-256:
`422ed77c943223fb5290eeb7fb601644ef597ca4d56da7135da67ab4f32cba4a`.

The [report archive](rendered-selection-dispatch-och17/reports.tar.gz) and
[verified manifest](rendered-selection-dispatch-och17/manifest.json) preserve
25 files of commands, logs, corrected failures, source hashes, patch reconstruction
and actual OS observations. These local checks establish no current-source Linux
acceptance, physical presentation performance or clean-machine distribution.

## Remaining qualification

Complete atomic-object selection/reveal cases, cancellation during competing user
interaction, reset/remount and independent installed-consumer OS scenarios. Cached
AX replay, visual-line adjacency and actual VoiceOver remain open. Catalog,
performance/resources, distribution, provenance and the full release gates also
remain required. OCH-17/OCH-41 and milestone 07 remain in progress.
