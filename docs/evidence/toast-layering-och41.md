# Layered toast bridge and gallery — OCH-41

Local integration checkpoint, 2026-10-03; macOS checkout, base `83eb87e` plus
the milestone working tree. This connects the measured
[presentation contract](../design/toast-presentation.md) through the public Core
API. Native enter/exit/reflow animation remains unfinished. Existing immediate
dismissal and expanded-column defaults are preserved.

`Toast.Stack.Layering.create` validates collapsed peek, expanded gap, fractional
width reduction and visible layers; `Toast.Stack.create ~layering` opts in.
Internal Op106 changes/reset only that metadata. Children, editor owners, event
handlers and active-time clocks retain their identities. Admission rejects invalid
values/kinds atomically and reserves fixed and per-submitted-card bookkeeping;
child-list edits participate in the same quota check. The 32-card bound remains.

The native widget measures every submitted live card at its actual current width,
then anchors the resulting geometry in the same frame. Expanded cards use their
individual heights; collapsed deep hidden cards contribute no footprint. Bottom
placement mirrors offsets and preserves the front edge even when content exceeds
the usable viewport. There is no previous-frame height feedback or recurring frame
clock. Scroll offsets are bounded and reset appropriately on collapse/expansion.

Hover or focus expands. A named Group provides a non-autofocusing keyboard and
accessibility entry point; Page Up/Down and Home/End scroll when that Group is
focused, leaving editor key handling intact. Back cards paint as decorations with
an identified hidden semantic ancestor, pointer shielding and Host input/Tab/IME/
expiry gating. Disabled front content remains exposed as disabled. Zero usable
area and empty stacks retire focus/expiry work. Removing or reordering cards
retains only application-owned keyed children. Old renderer listeners hold weak
state and an epoch, and cannot retain a removed owner.

Feedback adds **Layer notification cards** and **Show three sample notifications**.
The three samples have different content heights, remain until individually
dismissed, and use the existing placement/inset controls. Motion is not advertised.

## Validation

All commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless specified. Local logs are
`scratch/agents/root-20260929-m7-resumed/toast-layering-*.log` and `toast-widget-*.log`.

- Four standalone TestPlatform renderer tests: eight anchors, variable dimensions,
  hidden tall card, streamed wrapping on the first frame with no correction-frame
  jump, pointer/semantic shielding, disabled/Ending cards, hover footprint, bounded
  scrolling, mouse exit without a final move and retired listener epochs.
- Two production Host TestPlatform tests: hover and named-scope expansion,
  no autofocus, timer pausing, editor identity/focus through metadata reset,
  reorder and removal, keyboard scroll, oversized bottom anchoring, zero-area
  focus/expiry retirement and empty-stack quiescence. Previously queued editor
  callbacks are allowed to drain once; they must not renew work after teardown.
- Three Core expect tests cover constructor bounds, independent wire bytes and
  metadata-only reconciliation/no-op/reset. A paired Rust codec test checks the
  hand-built fixture, truncation, nonfinite/out-of-range values and trailing data.
- Admission verifies atomic wrong-kind/value rejection, configured reservation
  limits, child-list growth accounting, reset and complete resource release.
- Final full native library suite passes **735 tests, two existing private-D-Bus
  skips**. The full protocol suite passes **352 tests without skips**; sixteen
  targeted native admission/lifecycle/geometry/reflow tests also pass.
- Final strict all-target native/protocol Clippy (`-- -D warnings`), Rust format,
  full OCaml `dune build -j 2 @runtest @fmt examples/gallery/main.exe`, catalog
  structural audit and `git diff --check` pass.
- Fresh independent gallery consumer passes:
  `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
  --workspace scratch/agents/root-20260929-m7-resumed/toast-layering-installed-gallery`.
  It installs the package into a fresh prefix and builds the gallery against it;
  result is `run=False`, so this establishes installed-API/build compatibility,
  not a physical GUI walkthrough.

No OS windows opened for these checks. TestPlatform does not establish real
macOS GPU/input/IME/VoiceOver acceptance or Linux desktop qualification. Physical,
resource, CI and distribution gates remain under OCH-17; native toast motion and
remaining catalog work keep OCH-41 and milestone 07 open.
