# Rating source review

OCH-41. Source reviewed; independent color control is implemented locally.
Native color and current gallery/consumer acceptance remain open. This is not a
completed functional-equivalent row.

The source is `crates/component/src/rating.rs` at gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The unmodified Git blob is
[snapshotted here](sources/component-rating.rs.txt), with its SHA-256 and upstream
license in [the source manifest](sources/manifest.json). No upstream dependency
or toolchain revision changes as part of this review.

## Public behavior and differences

| Pinned source | GPUIO mapping and remaining work |
| -- | -- |
| `new`, `value`, `max` | `Rating.Config.create` and `View.rating`. Both support integer stars and zero as unrated. GPUIO validates maximum 1..32/value 0..maximum; the source accepts an arbitrary `usize` maximum, including zero, and clamps value in each builder. Bounded admission and atomic validation are intentional bridge policies. Neither implementation offers fractional ratings. |
| `on_click` and native keyed value | GPUIO sends ordered `Rating.Request` values to application state. Only hover preview is native-owned. No optimistic committed value or synchronous callback crosses the bridge. External config updates replace the displayed committed value without remounting. |
| Clicking a star at index `i` | The source emits `i - 1` if its rendered value is at least `i`; otherwise it emits `i`. GPUIO emits `Toggle i`; the default reducer clears exactly the selected index to zero, otherwise selects `i`. These policies are different. A caller can translate `Toggle i` to `Set (i - 1)` when its latest value is at least `i`, preserving current disabled/read-only/range checks through `Config.apply_request`. Do not silently change the established default reducer. |
| Hover | The source colors stars through the hovered index while retaining the committed filled-star shapes. GPUIO previews the hovered integer using filled/outline shapes, including previewing a lower value. Hover emits no application request and does not change the accessible committed value. This is a presentation difference, not a new committed value. |
| `disabled` | Supported, with additional read-only policy. Disabled leaves traversal; read-only remains focusable/readable. Current native policy and generation checks reject stale requests. |
| `with_size` / `Sizable` | `star_size` supplies explicit 8..128 logical pixels. The source sizes icons using shared size presets and adds per-star padding. GPUIO sizes each whole star box and uses the enclosing style for layout. Equal numerical sizes therefore need not have equal total width. Geometry should be tested against the documented GPUIO units. |
| `color` plus `Styled` foreground | The source supplies an independent active/hover color, retaining inherited foreground for inactive outlines. GPUIO's new `Rating.Appearance` supplies independent active/inactive colors, resolving theme tokens during View submission. Omitted components retain computed foreground defaults (inactive: 0.7 opacity); explicit components preserve their alpha. This closes the API gap locally; native color acceptance remains pending. See [appearance contract](../design/rating-appearance.md). |
| Root style refinement | Ordinary GPUIO root styles apply to the row: padding, sizing, border, background and state refinements. The root is a single focus owner; the per-star shapes are passive children. Colors and geometry still require actual native evidence for each added policy. |
| Semantics and keyboard | GPUIO adds a labelled slider role, committed value/range, one focus stop, relative arrows, Home/End/Delete/Backspace, and native AX increment/decrement/set-value actions. These are existing GPUIO contracts; source mouse handling alone is not evidence for them. |

## Existing evidence and its limits

The [OCH-33 rating checkpoint](../evidence/presentation-components-och33.md#controlled-rating-implementation)
records earlier actual macOS native and public Component Studio checks. The
current fixture is [rating_test.rs](../../rust/native/src/rating_test.rs): hover,
GPU fill/outline paint, key bursts, AX, readonly/disabled, pointer/modal gates,
maximum count, synthetic density, idle and removal. It does not prove the missing
independent color policy, a current installed-gallery run or physical monitor
transitions.

[Core expect tests](../../test/view_api/rating_test.ml) cover bounded construction,
ordered reducers, independent codec fixtures and callback/identity retirement.
Native admission and native rendering are separate checks; neither can substitute
for public Bonsai event delivery. No additional native Rating run was performed
during this source review.

The public gallery's Numbers & codes page now has size/maximum cycles, separate
colors, disabled state and an alternate step-down click policy. It still shares
the read-only reducer with the page. The new `--section rating` native driver
checks geometry, owner identity, keyboard/pointer requests, policies and page
retirement; a real macOS run now passes 41 geometry/identity cases, both themes
and native keyboard/pointer policies. See [the dated evidence](../evidence/numeric-disabled-rating-och41.md). The native presentation fixture also contains
GPU color/alpha/hover/opacity/inheritance/reset assertions, now passing in the
[local native suite](../evidence/rating-gpu-och41.md). These are scoped results,
not whole-release acceptance.

## Remaining implementation and acceptance

Local paired-bridge, public API, reducer and installed-consumer build checks now
pass; commands and scope are in the [appearance contract](../design/rating-appearance.md#local-verification--2026-10-01-utc).

1. Scoped physical gallery and native GPU checks now pass; preserve the
   [recorded coverage and limits](../evidence/rating-gpu-och41.md), including the
   distinction between synthetic density and physical monitor transitions.
2. The [fresh installed-gallery desktop walkthrough](../evidence/avatar-native-consumer-och41.md)
   now passes. Required final-source macOS/Linux hosted and broader release gates remain.
   Preserve earlier evidence at its recorded revision and scope rather than
   relabelling it as validation of the new appearance API.

Full Linux desktop qualification remains OCH-47; source review and deterministic
checks do not establish Linux GUI acceptance.
