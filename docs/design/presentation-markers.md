# Rich conversation markers

OCH-41. `Presentation.Marker` composes the pinned
[Marker source](../catalog/sources/component-marker.rs.txt) from gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271` using existing native primitives.
The original `Presentation.marker` dot/string helper is unchanged.
The source row is a locally validated functional equivalent on macOS. Other
catalog, platform and release acceptance remains open.

```ocaml
let module M = Presentation.Marker in
let content =
  M.Content.create
    ~key:(Key.of_string_exn "status")
    [ M.Content.Item.text ~key:(Key.of_string_exn "text") "Thinking · 京都"
      |> Or_error.ok_exn
    ; M.Content.Item.element
        ~key:(Key.of_string_exn "cancel")
        (View.button ~on_click:on_cancel "Cancel")
    ]
  |> Or_error.ok_exn
in
M.create
  appearance
  ~key:(Key.of_string_exn "thinking")
  ~variant:Separator
  ~loading:is_busy
  ~loading_style:Shimmer
  [ M.Item.content content ]
```

The result is an `Or_error`: root keys must be unique and must not begin with the
reserved `gpuio:marker:` prefix used for decorative lines and the implicit spinner.
`Content.create` separately rejects duplicate child keys. Text construction checks
UTF-8 and the native shimmer limit of 16,384 bytes even when loading is off. This
makes later loading transitions valid without changing the text's domain.

## Appearance and loading

Plain is a full-width muted row with minimum height 16px, gap 8px, font size 14px
and line height 150%. Separator centers content between flexible one-pixel lines;
Border adds a bottom border and 8px bottom padding. Root style refines these
defaults; `separator_style` refines each line. Icon defaults to a centered,
nonshrinking 16px square; Content has minimum width zero and is nonshrinking with
centered text in Separator mode. Each slot's explicit style refines its defaults.

| Content and loading | Behavior |
| --- | --- |
| Not loading | Ordinary text and styled rich content |
| Spinner, no typed Icon | Automatic 16px native spinner before caller items |
| Spinner with any typed Icon, including empty | Caller icon; no automatic spinner |
| Shimmer with typed text | Glyph shimmer on typed text only; rich siblings unchanged |
| Shimmer with no typed text | Entire rich Content pulses opacity by a factor from 0.6 to 1 |
| Empty typed text | Still counts as text; suppresses the rich-content pulse |
| Root-level arbitrary Element | Retains ordinary paint and interaction in every mode |

An arbitrary element that looks like an icon does not suppress the spinner; this
matches the source's typed slot distinction. `Marker.Spinner.create` configures the
localized accessible label, animation flag and period using the existing loading
bounds. Default label is `Loading`, period 1200ms, animated true. Shimmer inherits
`Presentation.Appearance`'s configuration unless explicitly overridden. Duration,
repeat and animation flag apply to both typed glyph shimmer and rich-only pulse.
Highlight colors, direction and spread apply to glyphs; an opacity pulse has no
spatial direction or highlight color.

The pulse uses two native ease-in-out stages across the exact configured duration,
including odd millisecond durations. The source uses cosine easing; this is a
functional smooth-pulse mapping, not identical samples of that curve. Once returns
to factor one; Loop repeats. `animated=false`, completion or changing to mixed text
uses a zero-duration one-shot factor of one. Reduced motion keeps the pulse at one
and glyph text ordinary; the spinner uses its existing static native indicator.

## Identity, styling and semantics

Each Content keeps the same keyed `Animation_program` root while static, pulsing
or containing typed text. Its style applies directly to that root. The native
[opacity factor](animation-opacity-factor.md) multiplies resolved base/interaction
opacity, including background and children; it does not replace a caller's opacity.
This avoids remounting controls or introducing a layout wrapper when loading changes.

`Item.element ~key` assigns the supplied view that sibling key using the public
`View.with_key` (also exposed in Bonsai). The view remains a direct flex child;
its grow, width, style, callbacks and children are preserved. This deliberately
replaces its previous outer key. Changing a key or view kind still replaces native
identity, as with any ordinary view. Caller keys remain stable when decorative
lines or automatic spinners are inserted, removed or reordered.

The root is presentational and adds no live region or focus stop. Apply
`View.with_accessibility` after successful construction to opt into a Status role
and the desired live priority. Every mounted GPUIO view receives a native ID;
explicit keys preserve identity across sibling edits. The spinner exposes a named
native ProgressIndicator, without automatic live announcements. Text retains its
logical source/accessibility name; rich children keep their ordinary control,
selection and accessibility behavior. Screen-reader acceptance remains in OCH-17.

Application/Bonsai state and tasks remain caller-owned. Native clocks drive the
paint effects; no OCaml per-frame timer or callback is added. Hidden/native-unmounted
content follows the existing animation/shimmer pause and disposal contracts. Page
remount creates new native owners; retained Bonsai model state can resume loading.
Each mounted Content uses one of the application's 1024 advanced animation owner
slots, including when static. Static programs request no ongoing frames. Use
managed lists to bound mounted content in large conversation histories. Existing
text/glyph/per-window shimmer work limits also apply; this adapter does not enlarge
those budgets or establish application-level performance acceptance.

## Validation status

Core expect checks pass UTF-8/size/key/spinner validation, typed icon and empty-text
policy, mixed versus rich-only effects, exact 1/3/2000/60000ms cycles and static
restoration. Twenty-four reconciler transitions preserve the direct rich control,
use its latest callback, keep equal snapshots idle and reject events after removal.
Independent style checks cover direct rich flex children, line/root/content overrides
and explicit versus default root semantics.

The public gallery has configurable variants, Spinner/Shimmer, None/Custom/Empty
icons, typed/rich/empty text, custom opacity/line styles, compact width and a retained
action counter. `scripts/test_gallery.py --section markers` is the focused native
check. Local macOS 14.5 arm64 results on 2026-09-30:

- The repository gallery passes 18 cases (two themes × three variants × three
  icon policies), 20 real OS Return/Space activations, retained native identity
  (`CFEqual`) and focus across configuration changes.
- Captured GPU pixels verify typed glyph shimmer with static rich siblings,
  rich-only opacity pulse, custom opacity, empty typed text suppression, stopping
  and native reduced-motion/static restoration. Default/refined separator lines
  and bottom borders paint correctly. Full-width/minimum-height geometry and
  interaction at compact width pass; compact width is not an exact size assertion.
- Page departure/remount retires native controls, preserves caller state, reports
  zero registered source bytes and resumes the pulse under full motion.
- A fresh installed-library consumer passes the same 18-case matrix, plus
  text-only shimmer and explicit rich-slot removal/reinsertion with new native
  identity and preserved action count: 21 OS activations total. Its final markers
  are `GALLERY_MARKER_OK` and `GPUIO_GALLERY_AX_OK: section=markers`, exit 0.

Both GUI runs use a 480-second process-group deadline, close their window and reap
its process. The first draft driver passed the dark matrix but looked up the
wrong theme-button label; correcting it to use the current theme fixed the
harness without a component change. Dark/light screenshots were visually reviewed.

Reproduction commands (run GUI checks with a bounded runner):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section markers --images scratch/marker-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-m7-marker-consumer-20260930
python3 scripts/test_gallery.py --section markers \
  --executable /private/tmp/gpuio-m7-marker-consumer-20260930/consumer/_build/default/main.exe
```

Full Dune build/tests/format pass. The final additional rich-slot checkbox and
text-only driver case were subsequently built/formatted and checked in the fresh
consumer. Core library code was unchanged after the full check. Structural catalog
and Python syntax checks are separate from behavioral acceptance. The consumer
uses staged public libraries, its own backend lockfile, the existing isolated
toolchain and repository native sources; this is not clean-machine distribution.
This adapter changes no Rust, protocol, fork or dependency code. The focused
section is included in the combined driver, but the entire gallery has not been
rerun at this checkpoint. VoiceOver, IME, Linux GUI, whole-application performance,
hosted CI and broader catalog/release gates remain separate.
