# Presentation separators

OCH-41. `Presentation.Separator.create` implements a labelled line using ordinary
retained Views. `component/separator` is a **functional equivalent, locally
validated on macOS**, with the explicit dash-spacing mapping below. This scoped
row does not accept the whole presentation family or release.

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
card exposes both axes, patterns, optional labels, width, custom colors and long
multilingual labels with explicit bounds. Its
focused driver is `python3 scripts/test_gallery.py --section separators`.
Initial native runs pass the dark-theme matrix (both axes, patterns, optional
labels and widths) with retained root/control identity, checked state and focus.
Dark solid/dashed screenshots were captured; the two dashed orientations were
inspected and show centered labels masking the line. Neither run completed the
light-theme matrix: one failed OS window capture while AX still exposed its
window; another lost AX window access, without a logged close request. No product
cause is established and these interrupted runs are not full passes.

## Installed-consumer acceptance — 2026-09-30

A fresh outside-checkout consumer stages the installed public libraries and
builds its own locked native backend without changing an opam switch or global
toolchain. On macOS 14.5 arm64, it passes all 32 theme/axis/pattern/label/width
cases and twelve long-label clipping/restoration cases. Checks include actual
centered geometry, the one-pixel unlabelled cross-axis size, retained root/control
identity, caller-owned checked state/focus, real Space input, readable label text,
page retirement/remount and clean exit. The close trace records the driver's
normal final `Window_close` request.

Long labels preserve their full accessibility source and natural text layout;
overflow clipping limits paint, not that source or those layout bounds. The
horizontal root is constrained to height 32; the vertical root to width 96.
Native captures sampled on a grid show visible text inside each clip and only
the frame background outside it in both themes. Removing each override restores
the original geometry. ImageIO/CoreGraphics decode those captured PNGs to RGBA8;
the test needs no optional Python image package. Four clipped captures plus
solid/dashed, horizontal/vertical theme captures were produced; representative
light-theme patterns and dark/light clipped text were visually inspected.
An earlier child-bounds containment assertion was corrected because it tested
layout shrinking rather than overflow paint. No production clipping change was
needed.

Reproduce with a fresh workspace:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-separator-review --run --gallery-section separators
```

The focused driver captures four clipping images even without `--images` (using
temporary files); `--images` retains them and the optional pattern captures.
Success markers are `GALLERY_SEPARATOR_OK` and
`GPUIO_GALLERY_AX_OK: section=separators`. Earlier interrupted runs remain excluded
from acceptance. Full VoiceOver, physical display changes and Linux desktop
qualification are not established by these checks. Required Linux builds/unit/
consumer checks, whole-gallery/hosted CI and OCH-17 release gates remain separate.
