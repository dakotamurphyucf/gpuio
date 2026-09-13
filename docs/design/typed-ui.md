# Typed UI API (OCH-8)

The `gpuio` library supplies pure, immutable descriptions through `Gpuio.View`,
`Style`, `Length`, `Color`, `Theme`, `Background`, `Shadow` and `Key`. It depends on
Core and the protocol, not Bonsai, Eio or a native runtime. `gpuio.bonsai` supplies
`Gpuio_bonsai.View`, whose button actions are `unit Bonsai.Effect.t`. The application
runner, scheduling and lifecycle integration are provided by `Gpuio_eio.App`
(see [runtime](runtime.md)); the working
`examples/view_api` executable currently supplies an explicit bridge/Eio runner.

## Components and types

A component is an ordinary function returning an `'action Gpuio.View.t`.
Applications choose action variants or Bonsai effects. `View.text`, `button`,
`row`, `column` and `grid` compose without raw node IDs or a syntax extension.
See `examples/view_api/components.ml` for reusable cards and a counter, and
`bonsai_component.ml` for a compiled Bonsai.Cont component using the same counter.

```ocaml
let counter ~count ~increment =
  Gpuio.View.row
    ~style:(Gpuio.Style.create_exn [Gap (Gpuio.Length.px_exn 12.)])
    [ Gpuio.View.text (Int.to_string count)
    ; Gpuio.View.button ~on_click:increment "+1"
    ]
```

`Width 10.` is a type error: Width requires `Length.t`. `Foreground "red"` is a
type error: Foreground requires `Color.t`. Mixing components whose callbacks
return different action types also fails at compilation. Smart constructors
validate remaining numeric constraints; `_exn` variants are convenient for
literal constants, and result-returning variants handle external input.

Lengths use logical pixels or percentages (100 means full size). Offsets and
margins permit negatives; widths and padding do not. Auto is available only where
GPUI permits it. Numbers must be finite and bounded. Theme entries are concrete
colors; missing tokens and token aliases fail before bridge submission. Gradients
have two ordered stops in 0..1 and a clockwise angle from the top. Shadows support
inset, signed offsets/spread and nonnegative blur, with at most eight per property.

## Composition, inheritance and native states

Properties are typed constructors, not string names. Shorthands expand before
composition. `Style.merge [defaults; overrides]` and repeated properties use the
last value for each individual field. `Style.unset style Property.Name.Width`
records a reset that survives merging over component defaults. Removing a field
from the next description also clears the previous native refinement: native
styles are rebuilt from the accepted complete style for that node.

Unset removes the local override and restores the primitive's default or inherited
value. It does not force a non-inherited initial value. Text color, font settings
and other GPUI text refinements inherit through containers. Theme tokens resolve
on the OCaml side; a theme change invalidates the shared-view shortcut and updates
resolved colors. Layout, borders and backgrounds follow GPUI's non-inherited
rules. Row/column/grid have explicit component defaults that can also be unset.

Native visual precedence is **base < value state < focused < hovered < pressed**.
OCH-11 adds checked/indeterminate/disabled state handling; see
[native controls](native-controls.md) for ownership, keyboard and semantic contracts. Use
`Style.with_state` for each state. Only focused controls match focused styling;
hover/pressed run in Rust without OCaml frame traffic. Keyboard input suppresses
hover according to GPUI input modality. Pressed currently follows GPUI pointer
activation; Space/Enter activate buttons but do not simulate pointer-held styling.
An unset state field reveals the lower-precedence value. Interaction properties
(pointer policy, selectable text, selection color and accessible name) are base
properties; state-specific use returns an error rather than being ignored.

Buttons have native focus handles, tab order, Enter and Space activation, a
button accessibility role and a name derived from their text. `?accessible_name`
overrides that name. Default padding, border, color and a contrasting focus border
can be overridden. This is a control baseline, not certification of screen-reader
support; full platform accessibility validation remains in its owning ticket.

`Pointer_events false` is inherited, with explicit descendant overrides. It
suppresses our click/focus-on-click, hover/pressed styling, cursor and text-selection
mouse handlers; it does not disable keyboard activation or native scroll handling.
It is not a general browser-style hit-test pass-through system. Use the future
input/overlay facilities for broader routing policy.

`User_select true` inherits into read-only Text nodes. Rust owns the selection and
clipboard operations; no per-drag callback crosses the bridge. Mouse selection,
shift-click, double-click word selection, triple-click select-all, grapheme-aware
Left/Right, Shift extension, Home/End and Cmd/Ctrl+A/C are supported. Selection
color inherits. Offsets are UTF-8 bytes and clamp to valid boundaries on text
changes. Native state is preserved for a stable node and discarded on removal,
replacement or deselection. This is per-text-node selection, not a rich-text editor
or cross-node document selection; editable text is implemented in its own ticket.

## Keys, events and commit ownership

Keys are unique among siblings. Keyed reorder retains identity; unkeyed nodes
use their sibling position. Changing key or primitive kind replaces the node.
Moving a key between parents is a replacement. Nodes and event bindings use
separate opaque generational handles; a removed node cannot receive a stale event.

`Reconciler` is the runtime adapter, not an ordinary component API. `prepare`
builds an immutable candidate against acknowledged state. Discarding it consumes
no identity. Submit its message, then `accept` only after native acknowledgement;
an update with no message is accepted immediately. There is at most one in-flight
update per window. Stale or foreign candidates fail. Even a callback-only update
refreshes the accepted OCaml callback without native traffic. Events for a live
binding use its latest accepted callback; closing the reconciler clears bindings.
All adapter operations and callbacks belong to one OCaml UI domain.

Shared unchanged subtrees are skipped when the theme is unchanged. Changed
parents compare sibling identity metadata in O(n); the child splice uses a common
prefix/suffix to transmit only the changed range. This is not an O(1) arbitrary
large-list reconciler. Initial updates remain bounded by the bridge's 4096
operations/1 MiB; managed lists and other retained components own larger workloads.
A 500-child append test transmits one splice at offset 500 and under 1 KiB total.

## GPUIX style coverage

Mapping against the committed GPUIX inventory in `api-and-parity.md` at
`18e695ed0ee8121a7793413ca795e08eda2a13df`. These are functional translations;
there is no promise of CSS syntax or the React development lifecycle.

| GPUIX fields | GPUIO property / behavior |
| --- | --- |
| display, visibility | Display, Visibility |
| flexDirection, flexWrap, flexGrow, flexShrink, flexBasis | Direction, Wrap, Grow, Shrink, Basis |
| alignItems, alignSelf, alignContent, justifyContent | Align_items, Align_self, Align_content, Justify_content |
| gap, rowGap, columnGap | Gap shorthand, Row_gap, Column_gap |
| gridTemplateColumns, gridTemplateRows | Grid_columns, Grid_rows: repeated equal tracks, not CSS template strings |
| gridColumnMin, gridRowMin | Grid_column_minimum, Grid_row_minimum: Zero/Min_content/Max_content |
| width, height, minWidth, minHeight, maxWidth, maxHeight | Width, Height, Min_width, Min_height, Max_width, Max_height |
| padding, paddingTop, paddingRight, paddingBottom, paddingLeft | Padding shorthand and Padding_top/right/bottom/left |
| margin, marginTop, marginRight, marginBottom, marginLeft | Margin shorthand and Margin_top/right/bottom/left |
| position, top, right, bottom, left | Position, Top, Right, Bottom, Left |
| background, backgroundColor | Background.solid or Background.linear_gradient (two stops) |
| color, opacity | Foreground, Opacity |
| borderWidth, borderTopWidth, borderRightWidth, borderBottomWidth, borderLeftWidth | Border_width shorthand and Border_top/right/bottom/left_width |
| borderColor | Border_color |
| borderRadius, borderTopLeftRadius, borderTopRightRadius, borderBottomLeftRadius, borderBottomRightRadius | Radius shorthand, Top_left/right_radius, Bottom_left/right_radius |
| boxShadow | Shadows: ordered list, including inset |
| fontSize, fontFamily, fontWeight | Font_size, Font_family, Font_weight |
| textAlign, lineHeight, whiteSpace | Text_align, Line_height, White_space |
| textOverflow, lineClamp, textDecoration | Text_overflow (Clip/Ellipsis), Line_clamp, Text_decoration |
| overflow, overflowX, overflowY | Overflow shorthand, Overflow_x/y (Visible/Clip/Hidden/Scroll) |
| cursor | Cursor; Move uses GPUI ClosedHand, the available drag cursor equivalent |
| pointerEvents | Pointer_events; inherited interaction policy described above |
| userSelect, selectionColor | User_select, Selection_color; native per-text-node selection described above |
| hover, active | Style.with_state Hovered/Pressed; Focused is also available |

## Validation

`test/view_api` covers callback-only refresh, keyed reorder/replacement, duplicate
keys, rejected/discarded/foreign updates, cleanup, theme changes, shorthand/reset
and bounded append output. OCaml and Rust independently construct all expanded
style tags and agree on `test/fixtures/style-v1.hex`; Rust tests every truncated
prefix and rejects invalid semantic styles atomically, including nested storage
accounting. Rust selection tests cover Unicode graphemes and clamping.

`cargo test -p gpuio-native --features native-tests --test native_ui` creates a
real window and injects native GPUI input. It checks grid bounds, native state
colors, mouse/keyboard actions, tab traversal, pointer policy, actual clipboard
copy, replacement focus identity and removed-style reset. Test-only paint probes
are absent from the ordinary native library. `examples/view_api/main.exe
--self-test` checks 20 acknowledged public-API commits and theme changes through
the actual OCaml/Rust bridge. macOS is the functional gate; Linux builds/unit
tests are required and Linux graphical runs remain informational under OCH-17.
