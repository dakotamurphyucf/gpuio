# Presentation separators

OCH-41. `Presentation.Separator.create` implements a labelled line using ordinary
retained Views. Native geometry/paint acceptance is in progress; this design does
not yet make `component/separator` an accepted catalog row.

The pinned [source](../catalog/sources/component-separator.rs.txt) exposes
horizontal/vertical axes, an optional text label and line color, solid/dashed
patterns and root styling. GPUIO maps those to `?axis`, `?label`, `?color`,
`?pattern` and `?style`. `?line_style` and `?label_style` additionally expose the
composed children to explicit, independent refinements. All are ordinary styles;
omitting an override on a later render restores the helper default.

```ocaml
Presentation.Separator.create appearance
  ~axis:Horizontal
  ~pattern:Dashed
  ~label:"Continue with another account"
  ()
```

The root centers one absolute, one-logical-pixel border edge. Horizontal uses
full width, vertical full height; callers must supply a bounded vertical parent.
An absent label gives the root a one-pixel cross-axis size. A present label
contributes its padded text size, wraps within the available width and paints the
appearance surface over the center of the line. For a different surrounding
surface, set `label_style`'s background accordingly. The default root clips
content to its bounds; ordinary root style can refine that policy.

The line uses the existing native border primitive with one top or left edge.
Dash spacing follows pinned GPUI border rendering. The source's custom path has
4-pixel dashes and 2-pixel gaps; those exact metrics are intentionally not this
API's visual contract. The functional surface includes both solid/dashed patterns,
not configurable dash arrays. There is no per-dash OCaml node, frame callback,
controller, native allocation class or protocol extension.

The root has Separator accessibility semantics, mapped to the platform separator
role; text remains a readable child. Neither root nor label introduces a focus
stop or action. Root/line keys remain stable across axis, pattern, label and theme
changes; removing the label retires only that child. Appending a separator does
not own neighboring control state. The original `Presentation.separator` keeps
its single background rectangle and existing custom-style behavior, avoiding a
silent reinterpretation of existing applications' background styles.

Two Core expect tests cover 32 theme/axis/pattern/label transitions, retained line
identity, idle repeat commits, independent root/line/label refinements and reset,
and the old helper's unchanged structure. The gallery's **Space with intention**
card exposes both axes, patterns, optional labels, width and custom colors. Its
focused driver is `python3 scripts/test_gallery.py --section separators`.
Initial native runs pass the dark-theme matrix (both axes, patterns, optional
labels and widths) with retained root/control identity, checked state and focus.
Dark solid/dashed screenshots were captured; the two dashed orientations were
inspected and show centered labels masking the line. Neither run completed the
light-theme matrix: one failed OS window capture while AX still exposed its
window; another lost AX window access, without a logged close request. No product
cause is established and these interrupted runs are not full passes.

Complete native geometry, pixels, long-label clipping and an installed-library consumer
still require evidence before functional-equivalent acceptance. Required Linux,
whole-gallery, hosted CI and OCH-17 release gates remain separate.
