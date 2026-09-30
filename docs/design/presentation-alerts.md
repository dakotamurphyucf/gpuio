# Rich alerts and banners

OCH-41 adds `Presentation.Alert`, preserving the existing `alert` and `banner`
helpers. The reference is gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, captured in the
[Alert source](../catalog/sources/component-alert.rs.txt). The source row is a
locally validated functional equivalent on macOS, including a fresh installed
consumer. Other catalog and release acceptance remains open.

```ocaml
let module A = Gpuio.Presentation.Alert in
let close =
  A.Close.create ~label:"Dismiss connection notice" ~on_click:on_dismiss ()
  |> Core.Or_error.ok_exn
in
A.create
  appearance
  ~variant:Warning
  ~title:(A.title "Connection interrupted")
  ~close
  ~visible:is_visible
  [ Gpuio.View.text "Your draft is safe. Try reconnecting."
  ; Gpuio.View.button ~on_click:on_retry "Reconnect"
  ]
```

The callback requests dismissal; the application updates `is_visible`. Actions,
Bonsai state, Eio tasks, document models and registered custom icons remain
application-owned. This composition adds no native controller, timer, FFI message,
capability, dependency or fork patch.

## Source mapping

| Surface | GPUIO contract |
| --- | --- |
| Default/Info/Success/Warning/Error | Typed variants; Default uses appearance foreground/surface/border. Other variants use accent/success/warning/danger, with alpha multiplied by 0.04 for the background and 0.3 for the border. Tokens resolve through the ordinary application theme; existing alpha is preserved. These are explicit GPUIO palette choices, not a reproduction of upstream Oklab/theme calculations. |
| XSmall/Small/Medium/Large | Horizontal/vertical padding and gaps are 12/6/6, 12/8/6, 16/10/12 and 20/14/12 logical pixels. Medium is the default. Radius is 8px, or 12px for Large. Font defaults to 14px with 150% line height. Ordinary inherited text and explicit styles still apply. |
| Card/Banner | Full-width flex row with a one-pixel border. Card shows the optional title and top-aligns content; Banner omits the title, removes radius and centers icon/body vertically. The pinned renderer still draws a border in banner mode despite its contrary doc comment; GPUIO follows the renderer. |
| Title/body | Optional rich title and arbitrary ordinary body children. `Alert.title` provides single-line text with ellipsis; custom title views retain their own content/layout. Body defaults to 3.2px child spacing, with 12px between title/body. Main/content containers clip overflow and allow shrinking. Large documents, paragraph styles and selection use the supplied ordinary views. |
| Icon | Default uses a semantic text glyph (information/check/exclamation/cross), Hidden omits it, Custom accepts any ordinary view including registered SVG. The source icon artwork is not bundled. Card icon has a 5px top margin; Banner has none. |
| Close | Abstract configuration validates a nonblank UTF-8 accessible name without NUL, at most 1024 bytes; no hard-coded English name. A native button displays a cross and retains ordinary pointer/keyboard/focus/disabled behavior. This intentionally improves on the source's click-only div. Optional styles refine the button. Closing sends the caller's queued action and never changes visibility by itself. |
| Style | Root, title, body and icon styles independently refine defaults. Close has its own style. The source's arbitrary rich body maps to ordinary GPUIO views, including document rendering; the helper does not parse Markdown itself. |

Visibility false produces an empty hidden root with no semantic metadata,
regardless of the supplied styles. Child native views and callbacks are retired;
showing again remounts them. The component does not retain hidden editors or tasks.
Stable internal keys preserve body and close control identity while changing
variant, size, icon or Card/Banner. Title removal or slot removal retires only that
subtree. The caller supplies stable ordinary keys inside lists when reordering.

The root uses Alert semantics with **Live.Off by default**. Applications may opt
into Polite or Assertive for meaningful notices; cosmetic changes do not rewrite
its metadata. The old helpers retain their previous Polite default and title
behavior. The new root adds no focus stop. Body controls and close keep their
native roles/actions. AX inspection and keyboard checks are not VoiceOver
announcement acceptance, which remains in OCH-17.

## Validation

Core expect tests cover close-label limits/localization, title/banner policy,
all four sizes, slot/style overrides, explicit live metadata, alpha-preserving
theme resolution and hiding. Forty variant/layout/size transitions retain both
body/close controls and current callbacks, keep equal snapshots idle, avoid
cosmetic semantic updates and reject late actions after hiding. Showing again
allocates new native controls.

The public Presentation card **A clear next step** exposes all variants/sizes,
Card/Banner, optional title/close, default/custom/hidden icons, disabled dismissal,
style refinement, compact width, restore and retained counters.
`scripts/test_gallery.py --section alerts` performs the focused check.

The initial repository run passes 20 theme/variant/banner combinations, eight
size layouts, default/custom/hidden icons, exact full/compact widths, single-line
title bounds, wrapped body growth, retained identities/focus, 21 OS body actions,
disabled close, one Space dismissal and one pointer dismissal. Slot removal,
hiding and page departure retire controls while caller state survives. Source
bytes reach zero on departure. The bounded 480-second runner exits 0, closes its
window and reaps the process. Its dark screenshot was visually reviewed.

A fresh installed-library consumer passes the same matrix plus independently
calculated GPU background/border color samples for both themes and every variant
in Card/Banner, and explicit border-color refinement. The first pixel reference
incorrectly blended the border directly onto the parent; GPUI paints it over the
alert background. Correcting the reference required no production changes or
wider tolerance. The final run reports `GALLERY_ALERT_OK` followed by
`GPUIO_GALLERY_AX_OK: section=alerts`, exit 0, and closes/reaps its process. The
light screenshot was visually reviewed. Both runs use macOS 14.5 arm64.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section alerts --images scratch/alert-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-m7-alert-consumer-20260930
python3 scripts/test_gallery.py --section alerts \
  --executable /private/tmp/gpuio-m7-alert-consumer-20260930/consumer/_build/default/main.exe
python3 scripts/audit_component_catalog.py
python3 -m py_compile scripts/test_gallery.py
```

Full Dune build/tests/format and the structural catalog/Python checks pass. Run GUI
commands with a bounded runner; these runs used a 480-second process-group deadline.
The consumer has a separate installed prefix/backend lockfile, using the existing
isolated toolchain and repository native sources; it is not clean-machine
validation. The focused driver is included in `all`/`core`; a new combined-gallery
run is still outstanding. Other catalog, hosted CI, required Linux non-GUI checks,
Linux GUI, VoiceOver, IME, whole-application performance and clean-machine
distribution acceptance remain separate. Full Linux desktop qualification stays
in deferred OCH-47.

Explicit styles on supplied children take precedence over inherited slot styles.
For example, change text-title weight directly with `Alert.title ~style`, or supply
your own title view; `title_style` controls the surrounding title slot.
