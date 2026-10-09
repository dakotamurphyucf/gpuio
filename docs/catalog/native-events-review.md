# Native event helpers — OCH-41

Reviewed 2026-10-04 against the original pinned Base `event.rs`, retained in
[the source manifest](sources/manifest.json). This module supplies dominant-axis
scroll filtering and a double-click convenience, rather than another widget
family or independent event loop.

| Pinned behavior | GPUIO mapping and evidence boundary |
| --- | --- |
| `OngoingScrollExt::lock_axis` / `InteractiveElementExt::lock_scroll_axis` | Native scrolling owns gesture-axis filtering and touch-phase state. `scroll::attach` sets the GPUI axis restriction; scroll owners keep/reset their `OngoingScroll` state. Carousel tracks have a native wheel owner too. Raw `Input_region` wheel observations preserve original axes, units and phases; they are not post-filter scroll offsets. The upstream WASM no-op is outside the macOS/Linux target. |
| `on_double_click` | `Input_region` click observations retain `click_count`, so application logic can choose exactly two clicks. No delayed OCaml result cancels the already-dispatched native event. Native semantic buttons/commands remain the keyboard/AX activation path. Window title-bar double-click is separately native and platform-dependent. |

Public [input-region behavior](../design/input-observations.md) includes capture/
bubble policy, raw keys, focus, wheel, occlusion and lifecycle. Captured gestures
use `Pointer` instead; file/typed drag/drop uses its own lifetime and payload
contracts. These functions must not be conflated with IME text input.

`input_region_test.rs` verifies double-click counts/modifiers and wheel phases/
units. Mailbox tests bound/coalesce eligible movement without losing discrete
events. The current native suite passed at the
[selection checkpoint](../evidence/window-selection-och41.md); detailed native
observations retain their [own evidence](../evidence/input-observations-och41.md).
The gallery's Input observations and Input & transfers examples demonstrate the
public API. Historical [scrolling checks](../evidence/scrolling-och11.md) and
[pointer checks](../evidence/native-pointer-och11.md) retain their actual scope.

The module mapping is explicit. Consolidated physical pointer/trackpad, focus,
IME, accessibility and current Linux nongraphical checks remain release work;
raw-event test success does not certify compositor behavior.
