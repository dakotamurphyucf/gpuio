# Native editor viewport commands

OCH-41 exposes `Gpuio.Editor_viewport` and asynchronous
`Gpuio_eio.Text_input.read_viewport` / `scroll_to`. Both ordinary input modes are
supported. Rust continues to own text, selection, composition and history.

```ocaml
let offset =
  Gpuio.Editor_viewport.Offset.create ~x:0. ~y:400. |> Or_error.ok_exn
in
Gpuio_eio.Text_input.scroll_to editor offset
```

Offsets are nonnegative distances from the document's left/top edges in logical
pixels, each bounded to 1e9. Native layout clamps them to the available extent;
single-line inputs clamp vertical scrolling to zero. A large valid offset can
request the end of the current extent. It does not promise to remain at the end
if later text extends the document.

`scroll_to` returns `Ok ()` when the exact native editor has accepted the request.
The offset applies on its next layout; acceptance is not a rendering or physical
presentation acknowledgement. Multiple requests before layout replace the pending
offset. Scrolling preserves selection/composition/history, does not focus, and is
permitted in read-only and disabled fields. It does not bypass stale window/node
identity checks. Subsequent caret movement can naturally reveal the caret again.

`read_viewport` returns `None` before the editor's first layout. Otherwise it
returns one coherent observation from the most recent completed native editor
layout/paint: offset, viewport width/height, line height, and a half-open range of
zero-based logical buffer lines. `first_buffer_line` / `buffer_line_limit`
include the engine's overscan; they are not an exact set of visible lines.
Wrapped display rows are not separate buffer lines. This observation may precede
pending text or layout changes. It has no physical-display timestamp and must
not be used as acknowledgement of an earlier scroll request.

The native adapter records the scroll offset in the same paint pass as the other
geometry. The live scroll handle can change immediately on wheel input before the
next layout; combining it with old geometry would create an inconsistent result.
The Base adaptation adds a small retained offset/accessor for this purpose.

Neither operation replaces the controller's last text snapshot or emits a fake
text change. Both share the existing 64-request editor budget and exact lease,
correlation, window-close and callback cleanup rules. Metadata replies of the
wrong kind fail with `Native_failure`. No polling, new per-frame subscriber or
synchronous Rust-to-OCaml callback is introduced. Applications query when needed;
a continuous viewport observation API remains separate work if required by an
application contract.

## Bridge

The unpublished epoch-3 editor command enum appends tag 8 `Read_viewport` /
`ReadViewport` and tag 9 `Scroll_viewport` / `ScrollViewport` (x/y float64).
The response enum appends tag 3 `Viewport` (optional geometry) and tag 4
`Viewport_scroll_accepted`. Geometry fields are offset x/y, width, height and
line height as float64, then the two buffer-line indices as bin_prot int64.
Coordinates are finite, nonnegative and <=1e9; line height is positive; line
indices satisfy 0 <= first <= limit <=262145. These additions do not change the
legacy text snapshot or public text-editing command result type. Rust's private
wire command/result types use PartialEq rather than Eq because they carry floats.

## Scroll retention repair

The pinned editor compared the IME caret's painted endpoint to a different saved
selection, so an unchanged composing caret appeared to move on every draw. That
pulled a manually scrolled viewport back to the caret. Masked text also compared
display offsets with logical offsets. The adaptation now compares and saves the
same logical caret key, including the IME endpoint, without modifying the public
selection. Native tests cover composition, masked Unicode, repeated redraws and
wheel input between layouts. This is separate from physical IME qualification.

The gallery's “Room to write” card demonstrates top/end requests and explicit
viewport inspection. See [validation evidence](../evidence/editor-viewport-och41.md).
Search/replace now has [typed commands and a reusable bar](textarea-search.md).
Source [range geometry](editor-range-geometry.md) now has an explicit snapshot-based
query and gallery inspector. Final platform/performance/release acceptance remains
open.
