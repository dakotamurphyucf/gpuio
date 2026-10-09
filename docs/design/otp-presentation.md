# OTP cell presentation

`Otp_input.Appearance` refines one retained segmented native editor. Core
`View.otp_input` and Eio `Otp_input.view` accept optional `appearance`. This is
separate from code policy, accepted value, provisional composition, selection and
history. Omitting the override or using `Appearance.default` restores the existing
presentation without replacing that editor.

```ocaml
let appearance =
  Otp_input.Appearance.create
    ~groups:2 ~cell_width:32. ~cell_gap:4. ~group_gap:20.
    ~background:(Color.token_exn "background")
    ~focus_border:(Color.token_exn "accent") ()
  |> Or_error.ok_exn
in
Gpuio_eio.Otp_input.view ~appearance code
```

Group count is 1–32 and is clamped to the current code length. Each group has
`ceil(length / groups)` cells, with a shorter last group when necessary. This
follows the pinned styled OTP grouping rule; empty trailing groups are omitted.
For example six cells with two requested groups gives 3+3; five cells with four
requested groups gives 2+2+1. Group gap replaces the ordinary cell gap at those
boundaries. No trailing group gap contributes to field width or the full-code
end caret.

Width defaults to twice the inherited font size, at least 28 logical pixels.
Explicit widths are 1–4096; ordinary/group gaps and radius are 0–4096, border width
0–64. All dimensions must be finite. Root View style controls field height, font,
foreground and outer layout. Border width and radius clamp to half the smaller
painted cell dimension. Explicit tiny cells can be smaller than their glyphs;
applications must choose compatible font and cell dimensions.

Background, border, focused border, selection and caret colors optionally use
theme tokens. Missing tokens fail OCaml reconciliation atomically. Omitted colors
preserve the existing translucent cell fill, inherited border/caret and blue
focus/selection. Focused border applies to all cells, retaining the existing field
focus treatment. Native caret blinking follows the separate
[caret lifetime contract](otp-caret.md); appearance changes do not restart its
phase or alter logical IME geometry during its off phase.

The native painter uses shared positions for glyph placement, cells, selection,
caret and point-to-byte mapping. During IME composition the provisional Unicode
draft stays one continuously shaped field; it is not split into ASCII cells.
Changing appearance retires previous hit/IME geometry until the new layout
paints. Geometry queries in that interval return unavailable, not coordinates
from the old grouping. The edit model, native owner, focus and history survive.
Caret visibility can adjust internal horizontal scrolling after a size change;
the API does not promise an unchanged pixel scroll offset.

## Bridge and ownership

The unpublished epoch-3 operation list appends tag 81
`Set_otp_appearance(node, optional appearance)`. Existing tags and OTP editing
config/command/event bytes remain unchanged. Both sides validate the bounded
presentation; the native tree permits it only on OTP nodes and charges the fixed
policy allocation to retained payload accounting. Invalid batches do not change
the tree revision, presentation or budget.

Fields, in order: group count as bin-prot int, optional float64 cell width, then
float64 cell gap/group gap/radius/border width, followed by optional RGBA int64
background/border/focus-border/selection/caret. RGBA values are 0–0xffffffff.
OCaml resolves theme colors before transport and sends only changed policy.
No synchronous Rust-to-OCaml callback, new request queue, native child input per
cell, vendor patch or capability bit is introduced.

The gallery's numeric page has live grouping and cell-size controls with masking
and read-only policy. Implementation and validation status belongs in the catalog
and evidence records; the existence of this design is not desktop qualification.
