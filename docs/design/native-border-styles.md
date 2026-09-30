# Native border styles

OCH-41. The shared style vocabulary now exposes the pinned GPUI renderer's
`BorderStyle::Solid` and `BorderStyle::Dashed`. This is the primitive needed by
custom Empty borders, dashed separators and pending attachments; it does not
complete those component contracts by itself.

## Public contract

```ocaml
Style.create_exn
  [ Border_width 1.
  ; Border_style Dashed
  ; Border_color (Color.rgb_exn 0x82cec6)
  ; Radius 12.
  ]
```

`Style.Border_style.t = Solid | Dashed` applies one pattern to the element's
border. Per-side widths, color and corner radii remain independent properties.
A zero-width edge stays invisible. No new native resource, OCaml frame callback,
pattern geometry collection or animation timer is allocated.

Base/focused/hovered/pressed layers follow existing style precedence. Last
property wins within a layer. `Style.unset ... Border_style` removes only the
chosen layer's declaration: removing a hover override exposes the base style;
removing the base exposes the receiving component's default (normally solid).
Explicit `Solid` can override a component-provided dashed pattern. Border styles
are not inherited by descendants.

Dash spacing and rounded-corner treatment follow the pinned GPUI renderer. The
API does not expose custom dash arrays or offsets. In particular, the pinned
[Separator source](../catalog/sources/component-separator.rs.txt) uses a 1px path
with 4px dashes and 2px gaps. The border primitive proves native dashed edges;
matching that source component still requires an explicit separator composition
and visual-contract decision. The pinned [Empty source](../catalog/sources/component-empty.rs.txt)
sets a dashed style but no default visible width.

## Wire and ownership

Field 67 appends `Border_style` after `Pointer_occlusion`; no existing tag shifts.
Its signed integer payload is 0 for solid and 1 for dashed. Native admission
rejects other values before publishing the transaction. A preceding valid text
change in the same rejected batch does not escape; revision and retained-byte
accounting remain unchanged.

Capability bit 49 (`CAP_BORDER_STYLES`) requires a matching host before any new
field can be sent by the current OCaml client. The current full mask is
`1125899906842623`. Independent OCaml/Rust fixtures specify:

| Payload | Hex |
| --- | --- |
| Solid field | `4300` |
| Dashed field | `4301` |
| Hello requiring border styles | `0001fc0000000000000200` |
| Hello requiring all current capabilities | `0001fcffffffffffff0300` |

The native mapper writes `gpui::StyleRefinement.border_style`; ordinary retained
views continue to own their identity and layout. There is no per-dash OCaml child
and no dependency or fork change.

## Evidence and remaining integration

Core tests cover paired field/Hello bytes, last-wins replacement, independent
widths, focused/hovered overrides and state-local unset. Rust codec tests cover
bounded request decoding and malformed/trailing data. Native unit/integration
tests verify GPUI refinement, malformed enums with atomic rollback, and session
negotiation including rejection of unknown/negative masks.

`native_border_style` runs a background window and reads actual GPU pixels through
the production retained renderer. Its 72 geometry/pattern cases cover solid,
dashed and omitted-property replacement at widths 0/1/3/8, square/rounded corners,
resizing, both axes and each individual edge. Additional GPUI-dispatched pointer
checks verify hover/pressed precedence and restoration. Removal releases all
retained tree bytes; a settled interval does not schedule additional renders.
This is GPU/style evidence, not foreground keyboard, IME, VoiceOver or Linux
desktop acceptance. The test closes and reaps its own window/process. Required CI
compiles the test on both platforms and runs it on macOS.

The public Styling details page includes a keyed border card with pattern,
weight and corner controls. The focused driver `--section borders` passes 16 native AX identity/geometry,
theme/pattern/weight/corner cases and state retention across page visits. The
normal-launch window closes cleanly. A preceding `--background` attempt did not
expose the gallery window to the AX driver and is not counted as a pass; the
background native fixture explicitly draws frames and is a separate test.
A fresh outside-checkout consumer also builds against staged installed libraries
and passes the same 16 cases with its independent locked backend. Both runs close
and reap the app. The broader consumer walkthrough and release gates remain open.

Local macOS arm64 validation also passes the full Dune build/expect/format suite,
235 protocol tests, 401 native unit tests, six session tests and three native style
integration tests. Strict native/protocol all-target Clippy passes with the native
image-test feature. Two private-bus tests are excluded from the normal macOS unit
run and remain required on Linux. Hosted checks have not yet accepted this change.

Reproduce with the repository environment and limited build parallelism:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --test style_values --test session --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --test native_border_style --locked -j2
python3 scripts/test_gallery.py --section borders
```

Empty's default and custom-border preview, the richer Separator API, Attachment
integration, full consumer walkthrough, whole-family/catalog and release gates
remain open. The shared primitive does not establish every specialized native
control's treatment of application-supplied border styles.
