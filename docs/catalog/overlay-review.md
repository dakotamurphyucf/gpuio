# Dialog, popover and help-surface source review

OCH-41, 2026-10-02. This review uses 24 unmodified snapshots from GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`: nine Base sources, fourteen styled
sources and the styled root initialization, including the eight-file `component/dialog` module. Paths and SHA256s
are in `sources/manifest.json`. They were extracted from the archive verified
against `third_party/sources.json`, not copied from our patched vendor tree.
This is a feature-level review, not whole-family acceptance.

## Dialogs, alert dialogs and sheets

| Pinned behavior | GPUIO mapping and evidence boundary |
| --- | --- |
| Dialog trigger, imperative handle, controlled open state, close reasons | Bonsai/application state controls `View.dialog` content. Ordinary buttons or commands open/confirm/close it; `Overlay.Dismissal` carries native Escape/outside-pointer requests. The application knows its own trigger/confirmation/imperative actions. There is no second native Boolean competing with the accepted content tree. |
| `on_ok`/`on_cancel` returns whether closing is allowed | Applications retain content while validation runs, and remove it only after accepting an action. Rust never waits on a synchronous OCaml approval callback. `on_dismiss` does not release the focus trap by itself. This is an asynchronous functional equivalent, not callback-signature parity. |
| Nested layers, focus trapping/restoration, topmost dismissal | Native `Focus_scope` and the overlay surface registry own ordering, modal gates and restoration. Existing controls/overlay tests cover nested deferred child popups and child-first Escape/IME. New backdrop tests exercise all six modal kinds with background pointer/AX rejection and forward/reverse Tab. Current physical qualification remains required. |
| Title, description, header/footer, icons, localized action labels, close button | Ordinary `View` content, accessibility heading/label metadata and buttons. Panel style supplies padding, backgrounds, borders, radii and typography. A separate Rust builder closure is unnecessary. Gallery dialog/drawer/confirmation examples are reusable public compositions. |
| Safe alert confirmation and no backdrop dismissal | `Alert_dialog.Config` forbids outside dismissal; the first eligible control is focused. Place the safe/cancel action first. Enter on the panel does not implicitly execute confirmation. This is an intentional safer default than the upstream Confirm action handler. |
| Backdrop theme / hidden backdrop artwork | Previously fixed half-opacity black. `?backdrop:Color.t` now supplies a theme-aware solid color for dialog/sheet/alert helpers, with transparent paint supported. Omission keeps the old default. Transparent paint never disables modal input blocking. Validation and native checks are recorded separately in the backdrop evidence. |
| Width / max-width / placement | Dialog width uses `Overlay.Config`, then ordinary panel style dimensions. Sheets use checked `Sheet.Config.extent`, edge attachment and native viewport clamping. Four edges are supported. Dialog absolute Position/Top/Left styles are verified by a native modal focus/backdrop regression. `Sheet.Insets` now reserves explicit app-chrome space inside the content viewport with all-edge attachment, proportional small-window compression and full-window modal blocking. The upstream custom window wrapper and 34-pixel title-bar theme are mapped as application-supplied layout, not inferred OS safe areas. See the [sheet inset contract](../design/sheet-insets.md). |
| Sheet `resizable(bool)` | The pinned component stores the flag but its renderer never reads it: no functioning drag-resize behavior is established by this API. Do not claim an upstream interactive resize implementation or add a release requirement based solely on the setter name. GPUIO sheets currently have application-controlled extent; draggable sheets require their own contract if added. |
| Sheet `overlay(false)` | The pinned sheet still installs a focus trap and an occluding visual overlay; the flag changes its extra overlay handler and close policy. It is not evidence of a fully nonmodal drawer. GPUIO transparency changes paint only; backdrop dismissal remains separately configurable. General nonmodal panel composition is ordinary View layout. |
| Entry transitions | The styled dialog animates opacity/vertical movement for 250ms; sheet slides for 150ms. `Overlay.Motion.Enter` now supplies opt-in native dialog/alert surface fade and slide (250ms), and all-edge sheet slide (150ms). It runs inside the deferred surface and respects reduced motion; immediate remains the default. Local tests cover actual paint, hit/AX geometry, retained identity, deferred autofocus and cleanup; physical visual acceptance remains open. Removed content is released immediately; no exit retention is promised. |

## Popovers and shared positioning

| Pinned behavior | GPUIO mapping and evidence boundary |
| --- | --- |
| Persistent trigger, lazy popup, controlled/default open, open-change notification | `View.popover` keeps the anchor mounted and mounts content when the application supplies it. Open state belongs to the application; dismissal requests are asynchronous. Accepted direct button anchors expose expanded/dialog-popup metadata through Op88. Trigger style and content are ordinary views. |
| Trigger rendering based on open state, click/Confirm toggling | Bonsai computes the trigger from its state and uses its normal button/command actions. Arbitrary trigger views do not gain guessed child-button semantics. Explicit secondary-button invocation can use native pointer events; no separate popover gesture owner is added. |
| Popup focus, Escape, outside click, restoration | Existing nonmodal focus scope, accepted dismissal policy, logical surface containment and generation-checked routes. Per-gap pagination also exercises popup-relative geometry, focus replacement and trigger restoration. See picker-trigger and pagination evidence. |
| Side/corner positioning, alignment, offsets, viewport clamp/flip | `Placement` exposes four sides, three cross-axis alignments and signed offsets. The shared native positioner measures current anchor bounds and clamps/flips. `Placement.at_point` adds all four corners with clamp-without-flip semantics; both strategies now accept a bounded viewport margin. Native paint/hit/AX/focus/resize tests cover popovers, help surfaces and nested menus. See the [placement contract](../design/placement-geometry.md). |
| Styled appearance flag, shadow/border/padding, rich content | Panel `Style` supplies ordinary styles and all content remains declarative. The upstream appearance setter comment claims it disables outside dismissal, but the actual render path forwards `overlay_closable` independently. GPUIO likewise keeps paint and dismissal policy independent. |
| Animated dropdown helper versus Popover | This source also defines a private `dropdown_popup` helper with reveal/opacity/shadow animation, but `Popover::render` does not call it. Do not infer automatic Popover animation from a colocated helper. Its dropdown callers belong to the corresponding choice/menu family review. |

## Tooltips and hover cards

| Pinned behavior | GPUIO mapping and evidence boundary |
| --- | --- |
| Text/custom tooltip content and style | `View.tooltip` accepts declarative content and a panel style. Content is retained while hidden to preserve models and native editors; unmount explicitly disposes it. This differs from invoking an upstream content builder during show/render. |
| Show/hide deadlines and rapid switching grace | Rust-owned bounded deadlines and per-window grace through `Tooltip.Config`. Defaults are 250ms show, 80ms hide and 300ms skip; delays are configurable, and there is no permanent idle polling. Existing tooltip tests cover grace, managed/controlled state, queueing and cleanup. |
| Action shortcut / explicit keybinding labels | Public `Command_binding` observations plus `Presentation.Kbd`, composed into tooltip content. The gallery command-tooltip example updates current bindings without replacing the action owner. No synchronous binding lookup into OCaml occurs during render. |
| Hover-card delayed trigger/panel pointer transition and open-change callback | `Hover_card.Config` and `View.hover_card`; managed state stays in Rust, controlled state follows the accepted application Boolean. Focus opens immediately; cards have no tooltip grace. Card content can accept focus, Escape yields to child native composition first, and closing focused content restores an eligible trigger without stealing outside focus. |
| Accessibility distinction | Tooltip help semantics differ from hover-card nonmodal dialog semantics. Hidden retained content cannot receive input or remain exposed to assistive technology. Existing native tests are evidence of the implementation; current public gallery/consumer and physical VoiceOver qualification remain open. |
| Mobile tap-to-open | Pinned Base switches behavior for iOS/Android. Those platforms are outside the accepted macOS/Linux scope; no desktop gap is inferred. |
| Styled help-surface motion | HoverCard uses the unanimated `render_popover_content` path. Root installs the managed Tooltip renderer, which defines a 150ms enter and a 200ms same-row switch transition using trigger bounds. `Tooltip.Motion.Enter_and_switch` now supplies opt-in native entry, same-row slide and cross-row immediate presentation. Managed replacement retains hidden content; controlled tips require accepted application changes. Native paint/input/identity/timing/cleanup tests and paired Op92 fixtures pass; physical qualification remains open. See the tooltip motion contract for the provider adaptation. |

## Remaining acceptance

Backdrop color, modal entry, tooltip transitions, point placement and explicit
sheet insets are implemented paths, not completion of these families. The
upstream custom-window infrastructure is intentionally not imported as an
automatic platform-safe-area policy.
The public gallery now includes live backdrop/motion controls, adjacent animated
help, point/corner placement, sheet insets and all four drawer edges. Run actual
macOS keyboard/IME/AX/VoiceOver, visual, nested-surface and resource checks.
Latest Linux compilation/unit/private-bus/consumer gates remain required; actual
Linux desktop qualification belongs to deferred OCH-47.

See [navigation contract](../design/navigation-components.md),
[navigation evidence](../evidence/navigation-components-och37.md),
[picker-trigger semantics](../design/picker-triggers.md#shared-popover-trigger-state),
[pagination evidence](../evidence/pagination-chooser-och41.md), and
[backdrop contract](../design/overlay-backdrop.md).

See also the [modal entry contract](../design/overlay-motion.md).

See the [tooltip motion contract](../design/tooltip-motion.md) and
[local evidence](../evidence/tooltip-motion-och41.md).

See the [sheet inset contract](../design/sheet-insets.md) and
[local evidence](../evidence/sheet-insets-och41.md).
