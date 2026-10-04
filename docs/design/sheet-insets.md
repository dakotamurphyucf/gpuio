# Sheet content insets

OCH-41. `Sheet.Config.create ?insets` accepts an abstract `Sheet.Insets.t`.
`Sheet.Insets.create ?top ?right ?bottom ?left ()` validates each edge as finite
and within 0..16384 logical pixels. All edges default to zero; `Sheet.Insets.zero`
preserves the existing edge-attached behavior.

Insets reserve space for application chrome inside the window's content viewport.
They are explicit application values, not inferred screen or OS safe areas.
For example, a custom app header can reserve 56 pixels at the top:

```ocaml
let insets = Sheet.Insets.create ~top:56. () |> Or_error.ok_exn in
let config = Sheet.Config.create ~label:"Inspector" ~insets () |> Or_error.ok_exn in
View.sheet ~config ~on_dismiss content
```

The sheet attaches to the chosen edge of the inset rectangle. Its extent clamps
to the rectangle and its other dimension fills it. The backdrop still paints
and blocks the **whole content viewport**, including the reserved band. Insets
do not make a modal header or background interactive. Dismissal remains a request;
the application must accept removal before the focus trap disappears.

When opposing insets would leave less than one logical pixel, they compress
proportionally to leave one pixel, or the entire viewport dimension when it is
smaller. Layout recomputes on native window resize without an OCaml update.
Arbitrary panel dimensions, margins, position and highlight-state geometry
remain subordinate to sheet attachment and size.

Padding and borders cannot enlarge a sheet beyond this box. The native adapter
measures the largest possible per-edge decoration across base and highlight
styles. If necessary, it scales padding and border widths together on that axis,
leaving one pixel for content. Using one scale across configured states prevents
combined hover/focus styles from breaking containment. Ordinary sizes/styles
are unchanged; original decorations return as the window grows. Percentage
padding uses the inset viewport's width, matching the containing-block rule.
In a tiny panel, content can be clipped; the modal scope remains keyboard
dismissable. Applications should still choose usable minimum window dimensions.

Inset and edge changes retain the modal frame identity and descendant owners.
They do not reopen the sheet, reset its entry clock or create timers. Entry motion
uses the current resolved size and position; accepted removal is immediate.
Focus restoration, native editor ownership and accessibility identity keep their
existing contracts. The gallery's drawer demonstrates reservation and cycling
through all four edges.

Op94 `Set_sheet_insets (NodeId, insets option)` adds four fixed-size floats in
top/right/bottom/left order. Defaults omit metadata; `None` clears it. Decoding
and tree admission reject malformed values atomically. A nonempty value requires
a FocusScope with one of the four sheet kinds in the final tree; removing or
changing that owner must clear the metadata in the same transaction. Storage is
inline in the native node and covered by its retained allocation accounting.

The pinned styled Sheet uses its custom `WindowBorder` insets and separately a
theme top margin defaulting to `TITLE_BAR_HEIGHT` (34 pixels) for left/right/top
sheets. Its bottom sheet has no extra top margin. `window_content_insets` returns
zero for server decorations and uses tiling-aware shadow/border widths for its
own client-frame wrapper. GPUIO now supplies a separate window-owned
[custom frame](window-frame.md). Sheets resolve inside its content bounds before
applying explicit sheet insets, so callers must not duplicate the frame shadow or
border there. Explicit insets remain application layout space, such as a custom
title-bar reservation; they are not an OS safe-area observation. Native macOS and
server-decorated windows receive no additional client-frame offsets.
Exact pinned window-border/title-bar sources and hashes are in the catalog.

See [local evidence](../evidence/sheet-insets-och41.md). Physical macOS visual,
keyboard/IME/accessibility and release qualification remain separate acceptance.
