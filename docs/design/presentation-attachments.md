# Rich attachments

OCH-41 adds `Presentation.Attachment` alongside the existing string-based
`Presentation.attachment` helper. The new adapter composes ordinary keyed views,
image resources, buttons and native text shimmer. It introduces no native widget
owner, wire message, capability bit, dependency update or upload service.

The behavior reference is gpui-kit revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, captured in the
[Attachment source snapshot](../catalog/sources/component-attachment.rs.txt).
`component/attachment` is a functional equivalent, locally validated on macOS
through the public application and an independently installed consumer. This
scoped row does not accept the whole presentation family or release.

## Application interface

```ocaml
let module A = Gpuio.Presentation.Attachment in
let title =
  A.Title.create
    ~key:(Gpuio.Key.of_string_exn "name")
    "design-notes.md"
  |> Core.Or_error.ok_exn
in
A.create
  Gpuio.Presentation.Appearance.dark
  ~key:(Gpuio.Key.of_string_exn "notes")
  ~status:Processing
  ~content:
    (A.Content.create
       [ A.Content.Item.title title
       ; A.Content.Item.description
           (A.Description.create
              ~key:(Gpuio.Key.of_string_exn "detail")
              "Markdown · 2.4 KB")
       ])
  ()
```

Applications supply `Status.Pending`, `Uploading`, `Processing`, `Failed` or
`Complete`. They own file access, Eio task scopes, progress, cancellation and asset
registration. The adapter never starts I/O or interprets status as a command.
`Size` provides XS/S/M/L and a validated logical-pixel basis. `Axis` selects a
horizontal or vertical card. Every slot and the root accept ordinary styles.

`Content` contains keyed `Title`, `Description` and arbitrary `Item.element`
values. Typed titles/descriptions inherit card status unless explicitly overridden.
Titles truncate and shimmer while uploading/processing; descriptions truncate and
use a translucent danger foreground on failure. Arbitrary children retain their
own behavior. Keys follow normal sibling uniqueness rules. Titles validate UTF-8
and the shared 16 KiB shimmer source limit even while static, so a later status
transition cannot fail because the source becomes too large.

`Media` has optional image, arbitrary children and overlay layers. Horizontal
media uses a 28/32/40/48px square (or a custom size); vertical media fills its width
with preferred aspect ratio 1. Explicit style dimensions and constraints follow
the [native aspect-ratio contract](native-aspect-ratio.md). Image fitting defaults
to Cover. Uploading, processing and failed status dim **only the image** to 60%.
Children and the overlay retain full opacity. Failed media with no supplied image
uses a 10%-alpha danger background. Supplying an image whose decode fails still
uses the image policy; applications can remove the image to select the fallback.
Image callbacks and resources follow `View.image` ownership and revision rules.

Pending cards use a dashed border; failed cards use a 30%-alpha danger border.
Descriptions use 80% danger alpha. The rest of the subtree stays fully opaque.
`Color.with_opacity` multiplies an existing alpha, preserves RGB and resolves
tokens against the current submission theme. Repeated factors are combined in
one bounded expression and alpha rounds once at resolution. Undefined tokens
remain errors even at zero alpha; theme definitions must still be concrete.

The default card radius is 12px for XS and 16px otherwise; media uses 4/6px. These
are GPUIO appearance defaults, not a promise to reproduce another application's
theme token values. Vertical card width defaults to 120px with content and 96px
without. Root/slot styles override these visual defaults independently.

## Activation and ownership

`Trigger.create` requires a stable key, nonempty UTF-8 accessible name and queued
action. It creates an ordinary native button covering media/content, with a focus
border and pointer, keyboard and accessibility activation. Its key belongs to a
separate internal slot. Changing status, theme, orientation or other slots retains
surviving owners and refreshes callbacks. Removing/recreating a slot retires the
old action generation. Disabled triggers neither activate nor request hover fill.

`Actions` paints above that trigger. Its children remain independent controls;
the cluster also shields gaps and disabled buttons against underlying pointer
activation. This pointer-only shielding is applied after custom styling. Style
validation already disallows interaction-property overrides in state layers.
Wheel propagation follows ordinary native scrolling. Vertical actions sit at the
top right, 12px from each edge; horizontal actions occupy their own row region.

With whole-card activation, put independent interactive controls in `Actions`:
the trigger covers media/content. Without a trigger, arbitrary content controls
retain their normal pointer behavior. This composition does not change the
single-target, passive-content restrictions on composed `View.link`.

`Attachment.group` is a horizontal scroll container with caller-owned child keys,
12px gaps and 4px vertical padding. It is not virtualized and does not remap
vertical wheel input into horizontal movement. Use the existing managed-list APIs
for large collections. Upload/task lifetimes do not depend on rendering a card.

## Animation and appearance

Built-in light/dark appearances carry explicit matching text-shimmer palettes.
The built-in dark configuration uses a white highlight: the source's default mix
toward the theme foreground produces no visible contrast when the title already
uses that foreground. This is an intentional GPUIO palette choice. Explicit custom
configurations retain their own highlight policy.
Custom `Appearance.create` retains the native default; callers can set a resolved
configuration with `Appearance.with_text_shimmer`. A card's `shimmer` overrides
that default; an individual title can override it again. Configuration and status
overrides are independent. Recreate explicit token-based shimmer configurations
when changing the application theme, following `Text_shimmer.Config`.

The [native shimmer primitive](text-shimmer.md) owns motion preference, visibility,
clock/wake scheduling and per-window work budgets. No Bonsai timer or per-frame
OCaml update is added by this adapter. Static configuration and reduced motion
render ordinary text. This reuse does not establish application frame-time or idle
resource acceptance on its own.

## Validation scope

Expect tests cover 100 theme/size/axis/status transitions, unchanged-view idle
diffs, retained source/action identity and current callbacks; slot retirement and
remount fencing; title/status/config precedence; image-only dimming; token/alpha
validation and repeated-factor bounds; explicit styles and pointer shielding.

The public Component Studio preview exposes those transitions, scoped decoded
image resources, independent actions, optional slots and explicit decode failure.
`scripts/test_gallery.py --section attachments` checks real macOS pointer and
keyboard actions, action gaps, disabled recovery, image geometry, native identity
and page teardown. These checks do not imply real screen-reader/IME, Linux GUI,
measured application performance or clean-machine distribution.

## Paint, motion and scroll acceptance — 2026-09-30

The additional `--section attachment-paint` check passes in a fresh consumer of
the staged installed public libraries on macOS 14.5 arm64. It uses actual native
window captures decoded through system ImageIO/CoreGraphics, not a mock painter.
In both application palettes it verifies moving title glyphs while uploading and
processing; ordinary static title pixels after failure/completion; the image's
60% opacity over its media background; undimmed opaque overlay glyphs; the
failed description's 80% danger alpha; the 10% no-image danger background; and
visible solid/dashed border differences. Native reduced-motion preference renders
the same title pixels as ordinary completed text; restoring full motion resumes
visible animation. These are application-native preference checks, not a change
to the owner's system accessibility settings.

The three-card preview constrains its group to 280px. Real two-axis macOS wheel
events verify horizontal scrolling when the pointer is in the action gap,
clipped-to-visible card movement, activation of the revealed card, retained action
identities, return to the initial offset and vertical propagation to the outer
page. The native one-axis container already uses GPUI gesture filtering and
restricted axes; Attachment adds no separate scrolling implementation. Image and
registered-source counters return to zero after page departure. The runner closes
and reaps its application.

The initial dark capture failed with zero changed glyph samples. The explicit
white highlight repairs that default, with passing captures in the public app
and installed consumer. A later capture-harness failure was eliminated by moving
full accessibility-tree identity queries out of the sampling loop: retaining the
text reference gives samples approximately 0.4 seconds apart instead of risking
aliasing the two-second sweep. The original source/geometry/static assertions and
pixel threshold remain in place.

The first scroll harness also incorrectly declared a variadic axis argument as
fixed on arm64. OS event readback proved that it emitted zero X delta. The final
driver uses `CGEventCreateScrollWheelEvent2` and asserts both actual point deltas
before posting. No native scrolling repair was needed. Failed runs are preserved
as diagnostics and are not counted as full passes.

Reproduce with a fresh destination:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-attachment-paint-review
python3 scripts/test_gallery.py --section attachment-paint \
  --executable /private/tmp/gpuio-attachment-paint-review/consumer/_build/default/main.exe
```

Success markers are `GALLERY_ATTACHMENT_PAINT_OK` and
`GPUIO_GALLERY_AX_OK: section=attachment-paint`. The companion `attachments`
section covers the wider status/layout/size/action matrix and decode-failure
recovery. Run both focused sections for Attachment acceptance; the paint section
starts with a fresh application's default preview model. The installed consumer
still uses this checkout's native sources/toolchain and is not clean-machine
distribution evidence. Combined gallery, required hosted macOS/Linux checks,
screen-reader and application performance/release gates remain separate.
