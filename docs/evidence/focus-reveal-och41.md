# Offscreen focus and ordinary scroll reveal — OCH-41/OCH-17

Local macOS continuation on `milestone-07-gallery-release`, based on `15e5aca`.
This is a focused fix and regression checkpoint, not completed Link-family or
milestone-07 acceptance. The advertised capability mask is unchanged.

## Problem and implementation

The composed-link gallery exposed a real failure: the first link could receive
focus, but later offscreen links were omitted from GPUIO's fallback Tab order.
Making every link visible in the test concealed that missing navigation behavior.

Focus records now include bounds and actual paint ancestry. Pure admission
projects each candidate through ordinary scroll owners, respecting clamping,
fixed clips, non-scrolling axes and the window viewport. Revealing a target uses
the resolved native overflow mask, including borders. Nested owners reveal the
visible portion from the inner owner; oversized controls align their leading
edge. A fixed-clipped or unreachable target cannot become an invisible stop.
The complete path is checked before mutating any scroll owner.

Reveal runs after complete paint on a focus change or explicit Tab/generic AX
focus request. Unchanged focus does not cause continuous scrolling or redraws.
Weak scroll references preserve immediate owner disposal. Floating deferred
surfaces use their actual paint ancestry, so dialog focus does not scroll its
logical anchor's page. Virtual-list/table viewports remain fixed boundaries for
this mechanism; their controllers still own materialization and logical focus.

See [the design](../design/composed-links.md#measured-focus-and-ordinary-scroll-reveal).

## Meaningful checks

`native_link` now includes `link_scroll_test.rs` through the production host:

- Nested X/Y owners, bordered clipping, offscreen Tab and Shift-Tab, signed stable
  order and ties, and actual target containment in both native overflow masks.
- Native range-slider lower/upper thumbs mixed with links and an ordinary button
  inside a modal trap, followed by focus restoration outside the trap.
- Manually moving native scroll offsets away from unchanged focus stays stable;
  explicitly focusing the same link through AppKit accessibility reveals it again
  without activation. This offset test is distinct from real wheel-event evidence.
- Fixed clips reject Tab and AX focus. Index-zero traversal cannot bypass that
  rejection. A vertical-only viewport excludes cross-axis overflow and negative
  content beyond its clamped origin. Oversized target geometry aligns correctly.
- After queued frames settle, two observation intervals verify unchanged whole-
  window render count. A floating modal takes/restores focus without moving the
  underlying scroll owners; teardown releases those owners.

The public `scripts/test_gallery.py --section links` uses actual macOS pointer,
Return, Space, Tab/Shift-Tab and AX actions with the public Bonsai/Eio app. It
covers eight theme/description/icon cases and 34 activations, retained native
identity, disabled recovery, scoped asset cleanup and shutdown. Every successor
focus step checks complete AX bounds inside the named Component preview viewport.
The test no longer manually scrolls later links into view. Both theme screenshots
were inspected; geometry and input assertions are the behavioral evidence.

## Reproduction and scope

Use the pinned isolated environment, with two build jobs:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-image-tests --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --test native_link --test native_scroll --test native_list --features native-image-tests --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section links
```

GUI checks need a local desktop and foreground focus. The development runs use
bounded process groups and reap each test before opening the next. Native tests
use GPUI-dispatched input and direct AppKit AX; the gallery uses macOS events.
Neither is a VoiceOver or physical-trackpad acceptance claim.

Final source validation passed locally: native Link/Scroll/List, the public link
gallery, full Dune `@all @runtest @fmt`, strict all-target native/protocol Clippy,
400 native unit tests, catalog structural audit and diff whitespace checks. Two
private-D-Bus unit tests remain intentionally ignored in the ordinary unit run;
their required isolated-bus/hosted gates are not claimed by this checkpoint.
Earlier in this change, controls, slider, document, table and input-region native
regressions also passed before the final overflow-mask/border refinement. The list exercised two full traversals of
100,000 logical records with at most 256 active views; the slider exercised 1024
owners/2048 thumbs. Those established regression workloads do not replace the
predeclared macOS release performance/resource audit.

Broader native compound/group ordering, passive content/style/race coverage,
fresh installed consumers, required hosted checks and release qualification
remain. Linux desktop qualification stays deferred to OCH-47; no Linux GUI
acceptance is inferred from these macOS results.

## Follow-up: non-stop anchors

At baseline `f8c2a22`, extending `native_link` to Tab after clicking an opted-out
link failed at the expected successor assertion. Fallback traversal had removed
the focused anchor while filtering destinations. The fix retains eligible
non-stops in the ordered records and searches one bounded cycle for a stop.

Local macOS `native_link` now passes that exact regression and the added
`link_nonstop_test.rs` matrix: signed/zero/tied indices, retained focus on policy
changes, pointer re-entry, both directions and wrap, all-nonstop trap fallback,
missing anchor and single-stop cycles. The main fixture also checks direct AppKit
AX focus on a non-stop; `link_scroll_test.rs` checks zero-index fixed-clip fallback
and a non-stop adjacent to native range-slider parts. Markers:
`GPUIO_LINK_NONSTOP_OK`, `GPUIO_LINK_SCROLL_OK`, `GPUIO_COMPOSED_LINK_OK`.

The test window closes and the bounded runner reaps the process on failure or
success. This is GPUI-dispatched input plus direct AppKit AX, not a VoiceOver or
physical keyboard/trackpad claim. Broader Link and release acceptance remain open.

The public gallery now exposes **Skip Release in Tab order**. Its external
`--section links` automation passes **eight theme/content/icon cases and 36
pointer/Return/Space/AX activations**, including AX focus on the middle non-stop
followed by Tab to its successor, pointer activation followed by Shift-Tab to its
predecessor, and Return activation while opted out. Successor bounds must fit
the actual preview viewport; identity, disabled recovery, scoped SVG cleanup and
shutdown still pass. This updates the earlier 34-action gallery checkpoint.

Follow-up regression checks pass locally: native controls, sliders and input
regions; 400 native unit tests (the same two private-bus cases intentionally
ignored); strict native/protocol all-target Clippy; full Dune build/tests/format,
then the gallery build/format check after adding the new toggle; and the source
catalog audit. Logs/reproduction commands use `GPUIO_JOBS=2` and the repository
environment. No new fork or protocol/capability change was required.
