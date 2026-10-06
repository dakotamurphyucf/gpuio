# Captured pointer sizing and typed drag/drop observations

[input_page.ml](input_page.ml) and its [interface](input_page.mli) demonstrate
native gestures with ordinary keyboard alternatives. Read payload fixtures,
Width_action/labels/tracing, state/lifecycle and pointer/transfer views.
`B = Bonsai.Cont` owns reactive state, `E` actions, `V` declarative GPUIO views and
`D = Drag_and_drop` validated immutable transfer values. No file is opened/read,
no provider task starts and no event history accumulates here.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe -- --trace-input
python3 scripts/test_gallery.py --section input
```

Choose Input & transfers. Drag the sizing track or use Narrow/Widen; move greeting/
custom card into the inbox and try Reject cards. The native driver automatically
requests tracing for this section. [README](README.md) qualifies actual platform
checks; no input/harness was run by this source review.

## Pointer capture and a validated application width

Width_action is Set/Adjust; `state_machine0` starts at 180 and clamps every candidate
into 80–360 logical pixels. Relative keyboard buttons reduce against latest width;
Reset assigns 180. `Pointer.Config` names the track and applies current disabled state.
Its native defaults capture Left press until release/cancel with synchronous prevent-
default/stop-propagation policy. `V.pointer_area` has stable key and 360×48 geometry.
Started/Moved/Released observations set width from local x plus notice; Cancelled
only reports the reason. Escape therefore ends capture without restoring an old
width. Coordinates can fall outside current region bounds and the reducer clamps.
Moved samples may coalesce; lifecycle edges do not.
[Pointer](../../lib/core/pointer.mli) supplies those contracts and no automatic slider
semantics, so ordinary buttons provide keyboard/accessibility sizing alternatives.

`let%arr` builds the resized 36-pixel-high panel and labels after each state change;
`E.Many` dispatches mutation/notice effects after native input. These callbacks
cannot retroactively change native capture policy. Departure clears gesture notice,
source notice and hover; retained width/drop count are independent of transient
feedback. Hiding/unmounting native regions releases capture.

## Payload snapshots and synchronous native acceptance

The text fixture is A small idea, ready to move plus emoji. Custom card has
namespaced kind org.gpuio.gallery.card/v1 and opaque bytes idea-42. Switching selects
one of these immutable values; a drag copies the payload at gesture start. Source
uses no files and does not opt into desktop-file offering. Desktop_offered/
Desktop_unavailable outcomes are handled generically; this fixture is primarily
an internal transfer source, not an OS copy/move guarantee.

Target allowlist always contains Text/Files and optionally that Custom kind.
Disabled state applies to source and target; accept decisions run synchronously
in Rust from current config, not in the asynchronous OCaml handler. No MIME
negotiation/deserialization follows a custom kind. A file path is native metadata,
not an Eio capability. Incoming paths can have unknown directory status and preserve
exact bytes. See [Drag_and_drop](../../lib/core/drag_and_drop.mli) for payload bounds,
gesture identity, external-offer limits and cancellation.

Source phases update latest notice. Target Entered/Left/Dropped/Rejected update
hover/receipt; Moved does nothing. Dropped calls `receive`, storing payload byte/file
count text and a saturating received-event count. One multi-file drop increments
once, not once per file. Rejected reports without counting. No full payload or
list of historical receipts is retained. Region colors derive from current hover.

## Diagnostic logging and explicit fallback action

With --trace-input, `trace_source` logs gesture/phase; `trace_target` omits Moved but
logs lifecycle, origin and file metadata. File paths are hex-encoded bytes rather
than assumed UTF-8 display labels, with optional directory status. Logging observes
native delivery, not reading or validating file contents. Received notices report
UTF-8 bytes for text and opaque byte counts for custom data.

Receive with keyboard or click directly invokes `receive`(payload), disabled under
current transfer/rejection settings. It provides an ordinary semantic action but
does not simulate a native drag lifecycle or prove native acceptance. Trace:
custom source starts → native target allowlist accepts → Dropped callback returns
hover-clear/`receive` effects → count/notice change → `let%arr` rebuilds inbox. Changing
format policy may retire/reconfigure an active gesture; the typed reason reports it.

To adapt, validate your own payload schema and retain explicit path authority before
file I/O. Put real storage/service work in an owned scope with current-generation
completion guards; do not perform it during native drag layout. Keep keyboard
alternatives and distinguish internal delivery from unconfirmed desktop outcomes.
Compilation and synthetic `receive` clicks do not establish physical/external transfers.
