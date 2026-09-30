# Rich tags

OCH-41 adds `Presentation.Tag` alongside the existing `tag` and `badge` helpers.
The reference is gpui-kit `84f57fdfcb4910623fb0bb7f795b077e249f9271`, captured in
[component/tag](../catalog/sources/component-tag.rs.txt). The source row is a
locally validated functional equivalent on macOS, using the public gallery built
as a fresh installed-library consumer. Other catalog and release gates remain open.

```ocaml
let module T = Gpuio.Presentation.Tag in
T.create
  appearance
  ~variant:Success
  ~outline:true
  ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 6.) ])
  [ Gpuio.View.text "Connected"
  ; Gpuio.View.button ~on_click:on_disconnect "Disconnect"
  ]
```

Children are direct flex children: there is no mandatory label, empty placeholder,
slot wrapper or automatic gap. Controls, images/icons, text and custom compositions
keep their own styles and interactions. Callers supply stable ordinary keys when
reordering. The root adds no role, action, live region or focus stop. Explicit
accessibility metadata can be added through `View.with_accessibility`. Removal
retires the affected native subtree; application state/tasks/resources remain
caller-owned. This helper adds no native controller, timer, protocol or fork patch.

## Source mapping and style contract

| Surface | GPUIO equivalent |
| --- | --- |
| Primary/Secondary/Danger/Success/Warning/Info | Typed variants. Primary and Info use Appearance's accent; other semantic variants use their corresponding color. Filled semantic variants use on_solid text. Secondary uses raised/foreground/border. Default Secondary. |
| Custom colors | `Tag.Palette.create ~background ~foreground ~border`, supplied through `Variant.Custom`. Colors can be ordinary theme tokens and retain alpha. Custom foreground and border remain unchanged in outline mode. |
| Named color scale | Supply an application-selected custom palette, optionally with named tokens for each color. Applications choose different palettes when changing appearance. GPUIO does not copy upstream's `ColorName` registry or automatic scale-50/950/200/800/300/600 selection. This is a color configuration mapping, not identical theme values. |
| Outline | Independent Boolean; clears only the background. Semantic foreground becomes its corresponding ink, Secondary becomes muted. Custom foreground remains caller-controlled. |
| Size | XSmall/Small share 6px horizontal and 2px vertical padding with 4px radius. Medium/Large share 10/4px padding with 8px radius. This matches the source renderer's two groups; Medium is default. Arbitrary custom dimensions use ordinary style. |
| Typography/layout | Native row, centered children, one-pixel border, font 12px and line height 125%. No fixed child width or gap; supplied styles refine defaults. |
| Rounded/custom corners | `Style.Radius` or individual corners. Radius 16px corresponds to the source's `rounded_full` (one 16px rem), without promising a pill at every arbitrary height. |
| Hover | Native Hovered opacity 0.9, applied before caller style. Explicit state style overrides it; `Style.unset ~state:Hovered Opacity` removes it. Base opacity alone leaves the hover declaration active. |

Opacity follows GPUI's paint semantics: inherited opacity scales each primitive,
including background, border and descendants, rather than flattening the subtree
into a single offscreen image. Overlapping border/background layers therefore
compose separately. Pinned `vendor/gpui/src/window.rs`, `paint_quad`, applies the
current opacity to both quad background and border. The pixel reference accounts
for this layering. There is no OCaml hover callback or per-frame state update.

Custom child styles override inherited text styles normally. In particular, choose
custom foreground colors that stay readable on the parent when outline clears the
background; the gallery uses its application palette in both themes. The old
string helpers keep their existing tones, size geometry, label slots and behavior.

## Validation status

Four Core expect tests pass seven-variant filled/outline mapping, custom colors,
size aliases, absent placeholder/root semantics, direct rich children, radius and
hover override/unset. Custom tokens preserve alpha and reject missing definitions.
Fifty-six variant/size/outline/reorder transitions retain the same native button,
use the latest callback, keep equal snapshots idle and reject retired actions.

The Presentation gallery card **Small details, useful actions** provides every
variant and size, outline, native hover default/override/unset, rounded corners,
child reorder, optional text and action, and a caller-owned action counter.
`scripts/test_gallery.py --section tags` is the focused native check.

The initial native run passed the dark palette/outline matrix and size checks,
then found an incorrect border-opacity expectation in the test reference. GPUI
composites its separately faded border over its faded background; the reference
was corrected without changing the renderer or relaxing tolerance. The complete
native result below is from the final installed consumer; the initial repository
run is only partial evidence.

On macOS 14.5 arm64, the fresh installed-library consumer passes:

- 28 combinations of two themes, seven palettes and filled/outline, with actual
  GPU background/border samples and retained native control identity/focus.
- Eight size measurements establishing the two size groups and four-pixel height
  difference with unchanged child content.
- Pointer-driven native hover default, explicit override and unset in both themes,
  with independently calculated pixels. Unset leaves base-opacity pixels unchanged.
- Reordered child geometry, rich-control-only composition, empty native AX group
  without a placeholder, removal/reinsertion with new native identity and retained
  caller counter.
- 30 OS Return/Space activations, page retirement/remount, and zero registered
  source bytes after departure. Source AX text retains the localized label.

The consumer reports `GALLERY_TAG_OK` and `GPUIO_GALLERY_AX_OK: section=tags`,
exit 0. Its 480-second process-group runner closes the window and reaps the process.
The light screenshot was visually reviewed. The preview's custom palette uses
application foreground/background/accent so outlined text stays readable in both
appearances. A constructor qualification error in that preview adjustment was
caught and fixed during compilation before the successful final consumer build.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-m7-tag-consumer2-20260930
python3 scripts/test_gallery.py --section tags \
  --executable /private/tmp/gpuio-m7-tag-consumer2-20260930/consumer/_build/default/main.exe
python3 scripts/audit_component_catalog.py
python3 -m py_compile scripts/test_gallery.py
```

The consumer uses an independent installed prefix/backend lockfile but the existing
isolated toolchain and repository native sources; this is not clean-machine
acceptance. Run native commands with a bounded runner. The focused section is wired
into `core`/`all`, but a new combined-gallery result is not claimed. Full Dune
build/tests/format, Python syntax and the structural catalog audit pass. Structural
catalog checks are not behavioral acceptance. Broader catalog, hosted CI, required
Linux build/unit/private-bus/consumer checks, VoiceOver, IME, application performance
and clean-machine distribution remain open. Full Linux desktop remains deferred
OCH-47.
