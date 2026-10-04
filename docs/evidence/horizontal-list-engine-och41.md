# Axis-aware native list foundation — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
This is the native foundation for the
[horizontal managed-list contract](../design/horizontal-managed-lists.md).
At this foundation checkpoint the public integration remained unfinished. The
subsequent [public integration evidence](horizontal-list-integration-och41.md)
records Core, paired bridge, Host, Bonsai, gallery and consumer progress.

The pinned GPUI ListState now supports an immutable axis through `new_for_axis`
and main-axis estimates through `with_uniform_item_extent`. Its existing vertical
constructor remains unchanged. A private coordinate mapping lets the same sum
tree/anchor/measurement/focus/tail algorithm handle both axes. Children retain
physical sizing, text orientation, paint/hitboxes and accessibility coordinates.
Public bounds and scrollbar offsets remain physical x/y. Wheel input uses the
configured axis and preserves child-first dispatch.

Initial estimates now survive first layout; cross-axis resize remeasures with
previous extents retained as estimates instead of collapsing distant unknown
items to zero. Public item bounds include leading padding, and scrollbar endpoints
include both main-axis pads. The tests exercise these with automatic and inferred
layout, unequal item extents and 100,000 logical items.

The focused-child autoscroll regression exposed missing accessibility rollback
in pinned GPUI's `Window::transact`: discarded nodes collided with the retried
prepaint. The fix restores open ancestor/focus state, removes newly completed
node IDs and reverses journaled focus/bounds writes. It does not clone the whole
completed accessibility tree. See
[adaptation details](../design/gpui-core-adaptation.md).

Review of the forthcoming axis reconfiguration path also exposed an existing
custom scrollbar ownership gap: the list adapter can replace ListState when its
configuration changes. The Host now closes the old presentation before replacement,
so capture/range focus/drag hooks retire against the correct handle. The next render
uses the replacement. Appearance metadata alone continues to preserve the owner.

## Native behavior evidence

Four TestPlatform tests cover both axes:

- 100k logical items with fewer than 32 native item renders in each explicitly
  checked frame, unequal extents, physical child/accessibility coordinates and
  physical scrollbar offsets; this is bounded-render evidence, not an RSS benchmark.
- Anchor preservation through cross/main-axis resize and streamed item growth;
  tail following through append and stopping on backward scroll.
- Perpendicular wheel no-op, child wheel interception, distant reveal and
  before/after viewport queries.
- Leading/trailing padding, inferred/automatic sizing, numeric scrollbar endpoint
  and actual last-item position.
- Child autoscroll preceding its own leading edge, accessibility retry with a
  focused child, final physical accessibility bounds and prepend retention.

The production list/tree scrollbar tests additionally change estimated height
20→40 during a live thumb drag. They verify old native drag hooks and OS-capture
state are cleared, old range focus retires, the current logical anchor survives,
the new range maximum is 3900 and End acts on the replacement handle. Assertions
read the current native owner after configuration changes, not a stale clone.

No OS windows were opened. Native TestPlatform input/semantic tree evidence is
not physical macOS keyboard/IME/VoiceOver/GPU qualification or Linux GUI acceptance.

## Commands and provenance

Use `GPUIO_JOBS=2 ./scripts/gpuio exec` for Cargo/Dune commands.

| Command | Result |
| --- | --- |
| `cargo check --offline --locked -j2 -p gpuio-native` | Pass at initial axis engine checkpoint |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib horizontal_list_test -- --nocapture` | Four focused tests pass before final focused-child assertion expansion |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib scrollbar_host_test -- --nocapture` | Three production Host tests pass with handle replacement coverage |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib` | Final 795 passed, two existing private-D-Bus skips |
| `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass after final handle-retirement changes |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` (direct) | Pass after final handle-retirement changes |
| `dune build -j2 examples/gallery/main.exe` | Pass after final handle-retirement changes |

The GPUI commit remains `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
The complete source patch was rebuilt against the hash-verified upstream archive;
its SHA256 is `225416601f67074d3c2abbed689770095773afd2074780e584e0a48da0062db9`.
`python3 scripts/vendor_gpui.py --archive scratch/agents/root-20260928-m7/zed-a57ba9b.tar.gz --output scratch/agents/root-20260929-m7-resumed/gpui-horizontal-ax-reconstructed`
and an exact `diff -qr` against `vendor/gpui` pass. These scratch paths record
local reconstruction evidence, not build dependencies. `sources.json` records
archive/patch hashes, and normal reconstruction can fetch the pinned archive.

The existing `block` future-compatibility advisory and duplicate system-library
link advisories remain. No shared switches or defaults changed. OCH-41 and the
complete milestone 07 goal remain open.
