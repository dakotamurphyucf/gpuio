# Scoped table scrollbars and cancellation — OCH-41

2026-10-03, macOS arm64; milestone worktree based on `83eb87e`.
This extends the [bridge checkpoint](scrollbar-bridge-och41.md).

The local table adapter now accepts an optional per-table native scrollbar
presenter. GPUIO uses it for the shared public `View.with_scrollbar` description.
The existing TableState and scroll handles retain ownership; the adapter keeps
vertical bars below headers and horizontal bars beyond pinned columns. Both
ranges reserve their common corner only while the sibling axis overflows.
Corner reservation changes track travel, not semantic viewport or numeric range.
Empty-body cached metrics cannot create a corner gap; all-pinned columns retain
only the vertical range. Reset restores Base presentation with the same table,
selection and offsets. Boolean visibility still suppresses the custom presenter.

Typed Host owner keys distinguish ordinary/list, table-horizontal and
table-vertical presentations. Callbacks retain weak widget references and run
only in Rust. Native geometry/input support a live sibling-handle corner without
copying offsets. Capture, focus and finite timing retain the existing widget
lifecycle. No public wire schema or library API changed in this checkpoint.

The table drag regression exposed GPUI dispatch ordering: a table's bound Escape
action executes before element key listeners. An ordinary-container test alone
had missed this. Scrollbar cancellation now runs in the existing window-scoped
keystroke interceptor before application overrides; it consumes unmodified Escape
only during a live scrollbar drag and preserves the table's cell selection.
The test setup also required matching `Set_table` and `Set_list_config` visibility
updates; the atomic admission rejection was correct and the fixture was fixed.

## Evidence

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless noted.

| Command | Result |
| --- | --- |
| `cargo check --offline --locked -j2 -p gpuio-native` | Pass at initial table adapter checkpoint |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib scrollbar -- --nocapture` | 40 focused tests pass before final empty/all-pinned assertions |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test scrollbar` | 791 library tests pass, two existing private-D-Bus skips; admission test passes, including final table assertions |
| `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` (direct) | Pass at table and final gallery checkpoints |
| `dune runtest -j2` | Full OCaml suite passes after gallery changes |
| `dune build -j2 examples/gallery/main.exe` | Pass before and after public gallery controls |
| `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-scrollbar-gallery-20261003` (direct) | Fresh staged packages and independent gallery link pass; `run=False`, no desktop launch |

The production table TestPlatform test checks header/pinned exclusion, disjoint
corner ranges, exact vertical numeric extent, keyboard End and AX SetValue on the
original handles, cell selection retention, drag preserving table focus, Escape
cancellation, live styling, axis removal, visibility off/on, metadata reset,
empty rows and all-pinned columns. Pure geometry/input regressions check sibling
overflow changes, zero-gap restoration, unchanged numeric endpoints and tiny
viewport clamping. A production tree test checks that range focus stays distinct
from tree-navigation focus while moving its real ListState and retaining its
anchor across visibility changes. The ordinary Host test also verifies Escape
while root focus is retained.

The dependency `block` future-compatibility advisory remains; strict first-party
lint passes. No OS windows were opened. TestPlatform verifies native dispatch,
layout and semantic trees, not physical input, VoiceOver, GPU paint or Linux GUI
acceptance. Public gallery controls and a fresh installed-consumer build now pass; physical
macOS qualification remains open; OCH-41 and the complete milestone remain active.

## Public gallery

`Scrollbar_preview` builds one checked optional description from public controls:
three visibility modes, three axis filters, reset to native defaults, gradient
thumbs with hover/pressed overrides, and finite slide/fade/width motion. The
Collections page decorates its existing managed list, tree and table outputs with
that description. A fourth tab renders an ordinary two-axis viewport with bounded
keyed row growth/reset. Theme changes rebuild the palette-derived description;
no internal scroll handles or bridge operations appear in the example.

`dune build -j2 examples/gallery/main.exe` passes with the new preview. The gallery
README records a manual walkthrough; that walkthrough is not yet physical input,
VoiceOver or GPU evidence. Native geometry and input are covered separately above.

The independent consumer stages public packages into a fresh temporary prefix and
links the gallery outside this checkout without installing into an opam switch.
Its result is `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The usual duplicate-system-library linker advisories are unchanged.
