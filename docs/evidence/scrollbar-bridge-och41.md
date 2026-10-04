# Scrollbar bridge and first production owners — OCH-41

Checkpoint: 2026-10-03, macOS arm64 working tree based on `83eb87e`.
The checked Core/Bonsai attachment, protocol and ordinary-container/managed-list
Host connections are implemented. **Managed-table rendering, public gallery
controls, installed-consumer validation and physical macOS acceptance are still
unfinished.** Tree roots use the managed-list path, but tree-specific focus/input
coverage is still required. OCH-41 and milestone 07 remain open.

## Contract and ownership

`View.with_scrollbar view config` accepts `Scrollbar.t option` on Container and
Virtual_list roots. It adds metadata without adding a layout wrapper or forcing
overflow. Core rejects other roots. Reconciliation compares the resolved wire
value, so theme updates emit a metadata operation while unchanged descriptions
emit nothing; None emits explicit reset. Existing children/node identity remain.

Op108 appends `Set_scrollbar`, with a checked option payload. Independent paired
attach/reset bytes are in `scrollbar-operation.hex`. Tree admission rejects wrong
kinds, invalid hidden-state refinements and budget overflow transactionally.
Retention reserves description/label bytes plus 8192 fixed native-state quota
units. These are conservative admission units, not measured RSS.

The Rust operation carries `Option<Box<Config>>`. Boxing keeps this sizeable
configuration from enlarging every update operation, including unrelated ones;
it is transparent to the paired serialized representation. The admitted tree
stores a shared immutable description independently of that transport allocation.

`scrollbar_host` retains native owners by node generation and presentation slot.
Live callbacks check current tree, modal/focus and pointer permission. Painted
range focus parts enter the Host's existing traversal/visibility machinery;
window focus preservation also recognizes their native handles. Reset, inert
state, removed owners and explicit window cleanup retire focus/timing/capture.

For ordinary containers, `scroll::Frame` lays out and paints the bar separately
from scrolled content. It uses the existing handle, filters axes from actual
computed overflow policy, and supplies the computed foreground/opacity. It
preserves existing wheel/reveal routing and resets a gesture lock when effective
axes change. For managed lists, the bar uses the existing ListState; a false
scrollbar flag suppresses custom presentation, and clearing metadata restores
legacy presentation. Native table metadata is currently admitted but not rendered.

The first production test caught a missing focus owner in Host's fallback check:
a scrollbar range was registered for focus but the next render moved focus to the
window root. Adding the range owners to that check fixes keyboard navigation.
An OCaml fixture test initially failed because the new copied fixture was missing
from inline-test sandbox dependencies; adding the dependency fixes that failure
without promoting exception output.

## Evidence

All commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` in the repository environment.

| Command | Result |
| --- | --- |
| `cargo test --offline --locked -j2 -p gpuio-protocol` | 358 passed |
| `cargo test --offline --locked -j2 -p gpuio-native --test scrollbar` | Admission/quota/rollback/reset test passes |
| `dune runtest -j2` | Pass, including all six scrollbar expect tests and the managed-root visibility-preservation assertion |
| `cargo check --offline --locked -j2 -p gpuio-native` | Pass at the first Host connection checkpoint |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib scrollbar_host` | Two production Host tests pass |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test scrollbar` | 787 library tests pass, two existing private-D-Bus skips; admission test passes |
| `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass after boxing the operation payload |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` (direct command) | Full OCaml/Rust formatting passes |
| `dune build -j2 examples/gallery/main.exe` | Pass, rebuilding the backend and linking the public gallery; no desktop launch |

The ordinary-container test checks that wide content cannot create a horizontal
bar when only vertical scrolling is allowed; numeric range, End-key movement,
Host focus permission, style updates retaining the same handle/owner/focus, inert
suppression and reset retaining the offset. The managed-list test checks End-key
movement of the real ListState and visibility off/on retaining its logical anchor
while retiring/recreating only the presentation owner.

These use GPUI TestPlatform. They prove native host/input routes and tree
semantics, not physical keyboard/IME, VoiceOver, GPU pixels or Linux GUI support.
No OS windows were opened. The gallery does not yet expose scrollbar controls.

Next: Host-level Escape cancellation while a drag preserves another control's
focus; scoped table renderer injection preserving headers/pinned columns; tree and
modal/clip/drag/removal coverage; a public gallery/consumer example. The widget's
own focused-range Escape route does not establish the Host-wide cancellation path.
