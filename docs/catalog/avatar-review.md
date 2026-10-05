# Avatar source review — OCH-41

Status: **reviewed with implementation gaps; not a completed source row**.
The group API and gallery example are now implemented locally; native group
and rich fallback acceptance remain pending. The rich slot is now implemented
locally; see its [contract and scoped tests](../design/avatar-fallback.md). An explicit identity palette
now has a fixed byte mapping and passing numerical audit.
This review uses the pinned Longbridge GPUI Kit revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The archive SHA-256 matches
`third_party/sources.json`; the following unmodified source snapshots and hashes
are recorded in `sources/manifest.json`:

- [Base avatar, image and fallback slots](sources/base-avatar.rs.txt)
- [Styled exports and size mapping](sources/component-avatar-mod.rs.txt)
- [Styled avatar, placeholder and identity colors](sources/component-avatar-avatar.rs.txt)
- [AvatarGroup](sources/component-avatar-avatar_group.rs.txt)

Existing [OCH-33 evidence](../evidence/presentation-components-och33.md#native-avatar-and-fallback)
proves specific leaf-avatar behavior. It does not cover the additional public
`AvatarGroup` or arbitrary placeholder/fallback content found in this review.

| Source capability | Current GPUIO contract / remaining work |
| --- | --- |
| Optional image | `Avatar.Config.create ?asset` uses existing scoped registrations and native image decoding. Registration ownership stays outside the value. |
| Image/fallback selection | The pinned Base picks an image slot whenever present, otherwise its fallback slot; it does not itself switch on image-load failure. GPUIO additionally paints its text fallback while loading or failed. Preserve that native behavior. |
| Name-derived initials | GPUIO accepts explicit validated initials/symbols through `Avatar.Fallback`. This deliberately avoids implicit name/locale assumptions. Callers may derive initials; do not silently replace the existing explicit contract with the source's ASCII-space splitting and byte-length special case. |
| Anonymous/custom icon placeholder | The styled source defaults to a User icon and accepts a custom icon; Base accepts arbitrary fallback children. `View.avatar_with_fallback` and `Avatar_group.Item.create_with_fallback` now admit bounded passive content. The gallery registers a custom person SVG; native GPU/AX/public-driver acceptance remains open. |
| Identity-dependent palette | The source derives one of 12 OkLCH hues from initials, with distinct light/dark foreground/background/border values. `Avatar.Palette` now provides an explicit deterministic 12-tone policy for keys or fallback text, using fixed opaque RGB pairs. Its minimum measured nominal text contrast is 4.789900:1. Caller overrides remain available; rendered glyphs and modified colors need separate checks. |
| Size and shape | Source named sizes are 16/24/48/80px, plus custom size; GPUIO's native leaf defaults to 32px and supports ordinary width/height/radius/font style. Exact defaults may differ, but named group sizing and custom size need an ergonomic composition policy. Preserve native clipping and fallback shrink-to-fit. |
| Root interaction | Source exposes native interactivity. GPUIO image observations are asynchronous. Existing passive link composition can make a labelled avatar a keyboard-accessible destination; a pointer-only input wrapper is not sufficient evidence for keyboard activation. |
| Group, overlap and limit | `AvatarGroup` is publicly exported. It takes the first `limit` avatars (default three), forces a shared size and overlaps later avatars by 30%. `Avatar_group.create` now supplies keyed grouping, shared size, limit and configurable overlap; the new gallery preview is compiled locally. Native overlap/paint/AX acceptance remains pending. |
| Group overflow | Source optionally appends an ellipsis when count exceeds limit, without a numeric count or own action. `Avatar_group.create` accepts a pure OCaml overflow renderer with exact omitted count/shared size. `Avatar_group.ellipsis` is passive with caller-supplied semantics; a custom native button owns its normal action. Zero limit unmounts all members; empty input calls no overflow renderer. |
| Identity and disposal | Current leaf source changes fence old image observations and retain leaf identity; changing fallback/label does not replace the leaf. A group must preserve this for surviving stable item keys, retire hidden-by-limit members, and define reorder/overflow lifetimes. |
| Accessibility | Current leaf exposes one labelled AXImage or is decorative, including fallback mode. A rich fallback must not add duplicate labels/focus stops or accidentally expose hidden descendants. Group reading order must follow the public logical item order even when visual overlap changes paint order. |

## Implementation direction

Keep `Avatar.Config` and `View.avatar` compatible. Separate passive leaf behavior,
rich fallback rendering and group composition rather than making the group own
asset loading, Eio tasks or a native controller registry.

The implemented [Avatar_group interface](../../lib/core/avatar_group.mli) depends
on `View` and `Avatar`. Putting this composition inside the existing `Avatar`
module would introduce a cycle because `View` already imports it. Its contract:

- A stable-keyed Item contains avatar configuration, optional visual refinements
  and its existing asynchronous image observer. Ordinary grouping adds no
  source/registration owner.
- A validated size policy supplies shared logical size, typography and overlap.
  Style precedence is explicit; group sizing must actually reach avatar roots,
  not only surrounding wrappers.
- Collection construction validates duplicate keys and limit bounds. It mounts
  only the visible prefix, so changes to the limit retire omitted resource leases
  and invalidate their observations. Restoring an omitted member is a remount;
  reordering surviving keys preserves identity.
- Overflow is a separate stable slot with an accessible description of omitted
  members. A caller-supplied action uses a normal native control. Do not silently
  interpret a decorative ellipsis as a button.
- Preserve input order in the retained children; later avatars overlap earlier
  ones using negative margins. This deliberately differs from the styled source's
  reverse paint order. No z-index or native callback is added. Actual geometry,
  paint, hit testing and accessibility order still require native validation.

The [rich fallback contract](../design/avatar-fallback.md) records the implemented ownership/rendering policy. Native image
readiness should select the visible fallback in the same frame, retaining the
existing failure/recovery and source-generation checks. A Bonsai callback that
later toggles a sibling is not automatically an equivalent replacement for that
contract. Bound any fallback subtree; explicitly decide whether it is passive,
which descendants/observers are allowed, whether hidden fallback resources remain
mounted, and how its semantics merge into the single avatar. Reuse scoped icon
assets and the normal retained tree; introduce no Rust-to-OCaml render callbacks.

The identity palette should have a stable, documented identity input and hash
algorithm if offered. Do not expose Rust's unspecified default hash as a durable
cross-runtime color contract. Verify actual sRGB contrast for every supported
light/dark palette pair, and keep arbitrary caller styling outside that guarantee.

## Acceptance still required

1. Core/codec/native tests for any added fallback representation and admission
   limits, including stale source/handler rejection and unchanged idle views.
2. Gallery examples for image/no-image/loading/failure/recovery, custom placeholder,
   group limits/overflow/reorder, sizes and both themes using public APIs.
3. Actual native geometry, clipping, GPU colors, overlap/hit testing and image lease
   accounting; no overlap assertion based only on expected style declarations.
4. Native labelled/decorative semantics, logical group reading order, real keyboard
   actions for interactive wrappers and no hidden fallback focus targets.
5. Group limit/reorder/source replacement/unmount/window-close cleanup, plus a fresh
   installed consumer. Older single-avatar evidence remains scoped to that leaf.

Required Linux build/unit/private-bus/consumer checks remain separate from the
macOS native acceptance. Full Linux desktop qualification stays in OCH-47.


## Local group implementation and evidence

`Avatar_group.Size` supplies the four presets and validated custom logical sizes.
`Item.create` borrows a config and optional image observer. The group validates
all item keys before taking its visible prefix, so omitted duplicate keys cannot
become an ambiguous mount later. Shared width/height/min/max/nonshrinking layout
and overlap margins apply directly to native avatar leaves after base custom
styles; caller font/color/corner styles remain available. State styles must not
replace structural fields. The overflow slot is outside the keyed item container,
so an item key named `overflow` is ordinary user identity.

Four Core expect tests passed with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
```

They cover finite size/overlap and limit validation, duplicate keys even at a zero
limit, exact omitted counts and no needless overflow callback, leaf sizing/margins,
logical child order, stable source-observer identities across reorder/size changes,
latest closures, limit retirement/remount generations and unchanged idle views.
The overflow action retains identity and its current omitted count even at limit
zero; removing/readding the slot fences its old queued presses.
These are pure construction/reconciliation assertions, not native GPU, physical
focus, accessibility reading-order or lease-accounting evidence.

The public Presentation page now includes a team preview with five members,
five sizes, three overlap values, limit zero through five, optional/actionable
overflow, reorder, rounded-square styling and scoped valid/invalid image sources.
`scripts/gallery_avatar_group.py` is wired to `test_gallery.py --section avatar-groups`.
The real macOS driver now passes 48 geometry/identity cases, both themes,
source/fallback transitions, reordering, limit retirement, overflow activation and
page remount after an intrinsic-layout repair. See [the dated evidence](../evidence/avatar-layout-och41.md).
This does not establish exhaustive GPU color/clipping or process-resource acceptance.

```ocaml
let item =
  Avatar_group.Item.create ~key:(Key.of_string_exn "ada") avatar_config
in
Avatar_group.create ~size:Avatar_group.Size.small ~limit:3
  ~overflow:(fun ~omitted ~size ->
    Avatar_group.ellipsis ~size
      ~description:(Image.Description.label (sprintf "%d more people" omitted)
                    |> Or_error.ok_exn)
      ())
  [ item ]
```

No new wire tags, Rust renderer ownership or dependency pins were required for
the group. Rich fallback is a subsequent native addition, with acceptance tracked separately. The optional identity-color policy is described below.


The complete local OCaml build/test/format check also passed after correcting a
gallery constructor qualification error:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-workspace>
```

The second command passed with `run=False`: a fresh independent gallery compiled
against staged installed libraries without modifying the project's switch.
This proves public API build compatibility on local macOS arm64, not execution of
the new group driver, GUI appearance, native resource disposal or Linux coverage.


## Fixed identity palette

`Avatar.Palette.for_key` maps an explicit stable key to one of twelve palette
entries; `for_fallback` applies the same mapping to explicit initials/symbols.
Neither helper derives initials from a name. Key bytes need not be text; fallback
bytes obey the existing validated UTF-8 contract. Hash the exact bytes with
FNV-1a-32, offset 2166136261 and prime 16777619, wrapping modulo 2^32 after each
byte, then take modulo 12. Case and Unicode normalization are unchanged. The
bucket is independent of appearance, architecture and Rust's default hasher.

The checked-in opaque RGB bytes are the v1 color contract. They derive from the
pinned source's twelve 30-degree hue steps and lightness/chroma pairs, using the
[licensed Oklab conversion snapshot](sources/component-theme-color.rs.txt),
binary64 arithmetic and nearest-byte quantization. GPUIO does not promise
bit-for-bit matching to upstream's f32/HSL roundtrip or its hash bucket selection.
No color conversion or hashing in Rust is needed at render time.

`Palette.style` sets background, foreground and border color only. It adds no
border width, geometry, image source, opacity, task or controller. Applications
choose when to apply it, and merge ordinary custom styles afterward. The gallery
uses the stable person key, preserves neutral image borders, and offers an
identity-color toggle; it does not rewrite the avatar's semantic name.

The nominal text contrast target is at least 4.5:1, following the
[W3C contrast criterion](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
The independent Python audit and Core test use the actual quantized bytes and
[relative luminance calculation](https://www.w3.org/WAI/GL/wiki/Relative_luminance).
The numerical audit's minimum across 24 light/dark foreground/background pairs is
**4.789900:1**. No rounding is used to decide the threshold. This is not a claim of
whole-application accessibility conformance, border contrast, rendered glyph
legibility, opacity-modified contrast or physical display calibration.

```sh
python3 scripts/audit_avatar_palette.py
```

The audit reproduces every stored RGB value and checks all text pairs; it is now
part of `scripts/gpuio lint`, hence the existing required lint jobs. A separate
Core test fixes ASCII/UTF-8/opaque-byte/maximum-key mappings, enumerates every tone,
checks contrast on public color values, and proves that appearance changes emit
only a style update while unchanged views remain idle. Color-only styles work
with an empty Theme and accept caller overrides.

### Rich fallback implementation constraints

The source review of `rust/native/src/image_view.rs` established a constraint:
SVG size/density requests may fail after the outer view observes its source.
Avatar SVG preparation now runs in prepaint, freezing the candidate for paint;
ordinary icon tint requests remain in paint to preserve native hover colors.
The [rich fallback design](../design/avatar-fallback.md) records native slot ownership,
prepaint selection, explicit rectangular overflow policy, TestPlatform evidence
and the unsuccessful earlier desktop-validation attempt.

The rich fallback does not replace that with a later OCaml
`on_change`-driven sibling swap. It must arrange native layout/prepaint/paint for
its bounded fallback subtree, preserve clipping, and choose visibility from the
current native image state. Admission and hiding must exclude hidden fallback
focus/actions and duplicate image labels. Resource lifetimes for hidden fallback
images/animations require an explicit retained/unmounted policy. The subsequent [rich-slot implementation](../design/avatar-fallback.md) supplies
native selection/ownership. Its TestPlatform evidence does not establish native
GPU/AX acceptance.


Palette validation on the local macOS arm64 checkout also passed the complete
`dune build -j 2 @all @runtest @fmt` check, including three new palette expect
tests. A fresh installed public-gallery consumer built successfully with
`run=False` after the palette API and identity-color gallery toggle were added.
No native palette/group driver ran: desktop Accessibility access remains
unavailable to this session.
