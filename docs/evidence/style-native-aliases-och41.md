# Native style aliases and layer resets — OCH-41

The [native value ledger](../catalog/gpuix-native-values.json) maps nine further
fields using GPUIX's native implementation, complementing the seven finite
TypeScript value sets. It is a reviewed keyword/API mapping with focused
refinement and codec evidence, not completion of the all-root style audit.

## Traceable source chain

GPUIX revision `18e695ed0ee8121a7793413ca795e08eda2a13df` uses a path dependency
on its `zed` submodule. That gitlink pins
[`remorses/zed` at `81c99f816b4a5f69d3c014774068034c24d1d7af`](https://github.com/remorses/zed/tree/81c99f816b4a5f69d3c014774068034c24d1d7af).
The unmodified GPUI `styled.rs` and its Apache license are now catalog snapshots;
manifest hashes and parent-submodule provenance are recorded alongside the
already pinned native GPUIX renderer. Git blob hashes were independently checked
against the parent tree and GitHub contents API. No dependency was upgraded.

The renderer's `apply_styles`, `Inherited::descend` and `selection_start_flag`,
and that GPUI revision's actual helpers, establish the following translations.
Do not infer semantics just from a CSS-looking name or TypeScript `string`.

| Field | Source aliases | Public choice |
| --- | --- | --- |
| `alignItems`, `alignSelf` | `start` / `flex-start`; `end` / `flex-end` | `Align.Flex_start`; `Align.Flex_end` |
| `alignContent` | same start/end aliases | `Distribution.Flex_start`; `Distribution.Flex_end` |
| `justifyContent` | same start/end aliases | `Distribution.Start`; `Distribution.End` |
| `alignContent` | `between` / `space-between`; `around` / `space-around`; `evenly` / `space-evenly` | `Space_between`; `Space_around`; `Space_evenly` |
| `justifyContent` | between/around aliases above | `Space_between`; `Space_around` |
| `position` | `absolute` / `fixed` | `Position.Absolute` |
| `textAlign` | `left` / `start` | `Text_align.Left` |
| `userSelect` | `text` / `auto`; `none` | `User_select true`; `User_select false` |

Remaining explicit choices (center/stretch/baseline where the particular source
handler accepts them, row/column and all three wrap modes) are listed in the
machine-checked ledger. GPUIO has additional typed values. The pinned GPUIX
`justifyContent` handler has no evenly branch and its direction handler has no
reverse branch; those do not limit GPUIO's existing richer API.

The justification difference matters: GPUIX calls `justify_start/end`, whose
pinned helper writes `Start/End`. Its items/content helpers instead write
`FlexStart/FlexEnd`; its self-alignment handler assigns flex-relative values
directly. GPUIO preserves these distinct native variants. `fixed` does not mean
viewport-fixed layout in the pinned GPUIX implementation. `textAlign: start`
is physical left in that implementation, not a new direction-aware API.

## Reset semantics

`alignContent: normal` calls `content_normal`, clearing `align_content` on the
current refinement. `Style.unset style ~state Align_content` removes that
layer's declaration, including earlier OCaml component defaults merged into the
same layer. It emits no native reset opcode. Removing a hovered declaration
leaves the base alignment active. Removing the base declaration does not remove
a hovered declaration; a subsequent declaration can override an unset marker.

This corresponds to the pinned helper's absent hover refinement, not a promise
that hovering can force a native default over an explicit base alignment. Nor
does an absent field necessarily remove an intrinsic default established by a
specialized native widget before GPUIO styles are applied. Such roots need their
own review. The public `Style.unset` documentation now states this boundary.

GPUIX's `text_decoration_none` helper only clears underline on its refinement;
it does not clear strikethrough or encode explicit inherited suppression. GPUIO's
existing `Text_decoration None` writes zero thickness for both. This is the
intentional explicit-replacement contract described in the
[finite-value audit](style-finite-values-och41.md), not an upstream bug claim.

`userSelect: auto` explicitly enables selection even below a disabled ancestor;
omission inherits. GPUIO ordinary text is opt-in, while GPUIX defaults enabled.
Native document defaults and editable widgets' ownership remain distinct. The
[selection audit](../design/selection-style-audit.md) records actual native
inheritance, cross-node/document selection, generation and teardown evidence.

## Pointer behavior stays separate

The pinned GPUIX `should_occlude`/`build_host_container` path uses
`pointerEvents: auto` to block pointer and wheel hitboxes behind the host, and
`none` to avoid adding that occlusion. Omitted values infer pointer-only blocking
from a painted background or absolute/fixed position. That path is not an
inherited listener-disable flag.

GPUIO deliberately separates `Pointer_events` (inherited listener eligibility)
from `Pointer_occlusion` (base-only native hit testing). `Pointer_and_scroll`
requests auto-style blocking; `Pointer` requests pointer-only blocking; `None`
restores ordinary native hit testing. It does not forcibly defeat an existing
native widget's own hitbox. Background/position do not silently add an occlusion
policy. This is an explicit functional API difference, not a one-to-one Boolean
translation of all upstream behavior. Existing pointer/wheel/keyboard tests and
public gallery evidence are linked from the
[input-observation contract](../design/input-observations.md).

## Evidence and remaining work

The OCaml expect suite locks alias field bytes and three independent layer-removal
cases plus later declaration replacement. Independent Rust encoding tests share
the byte contract. Native refinement tests compare alignment targets with GPUI
helpers and show that an empty or `content_normal` state refinement preserves
base alignment. Atomic transaction tests cover all valid enum choices plus
negative/upper-bound rejection with no source/revision/retention mutation.

The catalog verifier extracts actual `Some("keyword")` branches from the pinned
renderer and checks complete, unique mappings to real typed properties,
constructors, Boolean values or the single reviewed unset operation. Checksums
freeze the helper semantics for human review. These checks cannot prove visual
layout, mouse interaction, or correctness of every manually selected mapping.

Display/visibility/overflow values, numeric and shorthand policies, intrinsic
widget defaults, deferred/specialized roots, combined gallery/consumers and
OCH-17 release validation remain open. No new platform GUI acceptance is claimed.

Local macOS validation on 2026-09-29, using `./scripts/gpuio exec`:

- `dune runtest test/view_api -j2`: passed.
- `cargo test -p gpuio-native --lib style::tests --features native-image-tests
  --locked -j2`: seven tests passed.
- `cargo test -p gpuio-native --test style_values --features native-image-tests
  --locked -j2`: three tests passed, including independent bytes and expanded
  atomic rejection coverage.

No GUI window was needed or opened.
Strict native/protocol all-target Clippy (`--features native-image-tests --locked
-j2 -- -D warnings`), both language format checks, catalog verification and
`git diff --check` also passed. Hosted CI, GUI and release gates were not rerun.
