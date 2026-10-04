# Buttons: pinned behavior review

OCH-41 source review, 2026-10-01. Source revision is Longbridge GPUI Kit
[`84f57fdfcb4910623fb0bb7f795b077e249f9271`](https://github.com/longbridge/gpui-kit/tree/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/component/src/button). Seven source snapshots are recorded
in `sources/manifest.json`: `base-button`, the component button module and its
button, button-group, dropdown-button, toggle and private button-icon files.
Each was fetched at that revision, checked against its GitHub blob identity and
recorded with SHA256. The private icon renderer belongs to button presentation;
it is not another catalog family.

This is a behavioral gap review, not completion of the button family. The existing
[button-icon evidence](../evidence/button-icons-och11.md) covers a narrower surface.
The [selection review](selection-review.md) covers the shared toggle model and
semantic toolbar composition.

## Public surface and current mapping

| Pinned behavior | Current GPUIO equivalent and acceptance boundary |
| --- | --- |
| Base pointer, Enter/Space, semantic Click, disabled gating and retained focus | `View.button`, `icon_button` and command buttons use native focus and asynchronous activation. Existing native/button-icon evidence covers these paths; current whole-family macOS acceptance is still required. |
| Explicit accessible name and icon-only names | `button ~accessible_name` and `icon_button ~label`; decoration shares one action/name owner. Stable `Key`/Node identity replaces Rust element IDs. |
| Leading/trailing artwork | `Icon.Decoration` slots on ordinary and command buttons. These slots are deliberately SVG-only; they do not accept arbitrary content, a spinner or a progress circle. |
| Arbitrary base/styled children, text labels and custom rich artwork | `View.button_with_content` and `command_button_with_content` now check a bounded passive subtree, including Progress. One native owner renders and activates the content; nested interactive children are rejected. Local Core/native regression evidence exists; physical macOS and consumer runtime acceptance remain open. |
| Loading and optional loading icon | `Button.Config.loading` suppresses activation and hover/pressed presentation while retaining ordinary focus. Per-owner native/Core fences reject queued loading cycles without disabling shared commands. The public preview composes spinner artwork and normal-color dimming; physical/resource acceptance remains open. |
| Application-controlled `selected` presentation | Applications can supply conditional root styles. This is distinct from transient pointer `Pressed` and from an accessibility toggle value; it must not be mapped automatically to `Command.checked`. The Actions, in context card changes root appearance independently of toggled metadata; its desktop driver checks that the owner remains AXButton. That driver is unrun. |
| Optional `toggled` semantic value / styled Toggle | `Command.checked` plus `View.command_button` supplies pressed/toggled semantics and current-model reducer behavior. The Controls formatting example exercises it. Rich command-button content is now supported locally; its toggled composition still needs family acceptance. |
| Default/primary/secondary/danger/info/success/warning/ghost/link/text/custom appearance | Public Style/theme fields can express colors, padding, typography, shadows, borders and hover/pressed/focus states. Dedicated named variants are not required to reproduce those styles. The Actions, in context card now demonstrates these palettes, outline, compact/large/rounded settings, selected appearance, disabled state and tooltips using public styles. Its Link example uses `View.link`. That API now adds focus-preserving loading independently of disabled and Tab policies in unpublished epoch3. The preview has a separate Link loading toggle; physical and consumer runtime acceptance of that extension remains open. Desktop visual/input acceptance remains open. |
| Outline, compact sizing, explicit size, rounding and per-edge seams | Public style values support presentation composition. The Controls gallery now composes connected and separated groups, both orientations and single-child geometry with existing style fields. The desktop driver includes adjacency/identity checks but is unrun; GPU corner/seam measurements and split-button coordination remain open. No ready-made group helper is exposed. |
| Custom signed Tab index and skipped stop | `Button.Focus.Focusable Tab_order.t` exposes signed order and skipped stops on ordinary and command buttons. Native TestPlatform coverage passes; actual macOS input/AX coverage remains open. |
| Base `focusable=false` auxiliary action | Separate from `tab_stop=false`: the source leaves focus on an existing sibling and gives up Enter/Space activation on the auxiliary button. `Button.Focus.Preserve` now expresses this policy; its native TestPlatform regression distinguishes it from skipped Tab. Physical input/AX acceptance remains open. |
| Role override | GPUIO intentionally exposes typed semantic components rather than arbitrary role mutation. Button/Link and command toggle behavior cover specific roles; arbitrary `RoleOverride::None` or unrelated widget roles are not promised. |
| Plain/rich tooltip, placement and command shortcut hints | `View.tooltip` accepts an arbitrary anchor/content and typed tooltip configuration. Command hints can use the binding-observer API and keyboard presentation. The Actions, in context card composes plain and rich tooltips with Top/Right placement; its driver covers focus/Escape and is unrun. The Hints that follow the action card now composes a stable command button, managed tooltip, mounted binding observer and observed keycaps; it supports shortcut replacement/removal, disabled commands and both display platforms. Build evidence exists; its physical walkthrough remains unrun. |
| Hover notification | `View.with_hover` now adds a separate generation-checked observer on Button/CommandButton/Link roots, preserving action and focus owners. Native listener, Link loading, command availability, ancestor/modal gates, keyboard modality, eviction and bounded-queue checks pass locally; the public appearance card displays hover and an independent installed gallery builds. Full policy, physical and consumer runtime acceptance remains open. Pointer captures and tooltip-open events remain distinct. |
| ButtonGroup single/multiple selection | Public command scope plus a row/column and stable-ID application reducer can express the behavior. Upstream reports rendered child indices; GPUIO should keep current-model ID-based requests across the asynchronous bridge. The Controls example covers single/multiple selection and connected/separated group presentation in both orientations, including single-item groups. Independent busy/disabled children and rich labels are now demonstrated with current-model reducer tests. Physical geometry/input acceptance remains open. |
| Styled ToggleGroup / segmented arrangement | The styled wrapper projects each toggle's current Boolean into a returned Boolean vector; the base group itself is a semantic toolbar. GPUIO's current-model command reducer is the appropriate asynchronous equivalent. Connected corners/seams, rich content and child-versus-group disabled/loading policy are composed in the gallery. Actual macOS input, rendering and lifecycle acceptance remain open. |
| DropdownButton action-only/menu-only/split | `View.split_button` now composes ordinary/command and MenuButton owners with bounded individual Tooltip anchors, native shared hover/menu-held paint, independent part policy and keyed mode retention. The public Controls preview demonstrates split/action-only/menu-only forms. Local Core/native checks pass; complete lifecycle, physical and installed-consumer runtime acceptance remain open. |

## Ownership and implementation direction

Keep one Rust-owned focus/action handle for each native button. A rich passive
label must not introduce another action target, selectable region or callback.
Reuse the established bounded passive-content validator and retain keyed label
resources through updates; remove decoration accessibility from the semantic tree.
Draft Core/Bonsai types before extending the button tree contract.

Loading policy needs an explicit distinction from disabled policy. In particular,
blocking one busy command button must not disable the same command everywhere in
the window. Retire or fence queued callbacks against current per-owner state, keep
spinner clocks paint-driven and apply reduced-motion/hidden/unmount/close rules.
Do not implement loading by ignoring requests only in the example's OCaml callback.

Groups should submit stable item IDs/intents and reduce against the current model,
including current enabled policy. Do not copy rendered Boolean vectors across the
asynchronous bridge. Pure group geometry and named appearance examples can stay
in OCaml; native ownership and input policy belong in the existing adapter.

The positive Int64 legacy mask is full. The [button extension](../design/button-content.md)
introduced mandatory Op69 in paired exact protocol epoch2. The current bridge
requires epoch3 for MenuButton observations/placement and rejects epochs1–2.
No checkable bit is broadened or sign bit reused. Future wire changes need an
explicit paired version decision.

## Required evidence before completion

Add a public button gallery covering rich content, loading, appearance/state
precedence, selected versus toggled, icon-only naming, group selection/presentation,
tooltips and split actions. Include defaults/reset and independent command owners.
Verify current-model callback fencing, focus identity, non-stop versus non-focusable
policies, native pointer/key/AX activation, ancestor disabled/inert/modal policy,
virtual-row eviction and resource cleanup. Match codec/rollback tests to any wire
extension. Real macOS and independently installed runtime checks remain required;
compilation and this source review do not close OCH-41 or OCH-17.

## Connected gallery composition

`examples/gallery/selection_preview.ml` keeps application-owned stable-ID
selection and composes group presentation with ordinary public style fields.
Connected mode retains all edges/corners for a singleton, rounds only the outside
corners of multiple buttons, and paints each shared seam once. Vertical groups
stretch buttons to a common width; horizontal groups share a height. Separated
mode restores individual corners and gaps. Group disable remains command
availability plus the current-model reducer guard. Filtering to one item keeps
the current alignment and Bold owners; hidden formatting selections survive
restoring the full group. No native group entity, callback vector or protocol
operation is added.

The public selection driver checks actual AX geometry for zero or six-point gaps,
alignment, singleton owner retention and restored selections. It is authored,
not executed: building the gallery and compiling Python cannot establish these
assertions, physical corner rendering or full button-family acceptance. The separate appearance preview demonstrates named palettes and plain/rich
tooltips. Per-child policy combinations now have a public composition and reducer
coverage; live shortcut hints also have a public composition. Full hover and
split-menu lifecycle coverage and physical acceptance of these previews remain open.

## Appearance and live pointer-state checkpoint

`Button_appearance_preview` demonstrates eleven variants, including a real
semantic Link, plus outline, compact/large dimensions, rounded corners,
application-selected appearance and disabled controls. Palette colors vary with
the gallery theme. Primary has rich help placed to the right; the other help
surfaces prefer the top. Actions only count preview requests. The button section
of the public driver now exercises these actions, sizing/identity, link roles,
rich focus help/Escape, Return/Space, disabled requests, themes and retirement.
It is authored and Python-compiles, **not desktop-validated**.

A production-View TestPlatform regression inspects painted quads through actual
native pointer dispatch: base → focused → hovered → pressed, then loading while
held. Loading restores focused paint, preserves the owner and rejects release;
blur shows base paint, disabled state overrides interaction colors, and restoring
availability retains identity and activation. This tests renderer state
precedence, not every preview palette, OS input or GPU pixels.

The [menu coordination design](../design/menu-observation.md) records the
implemented subscription/placement contract and remaining shared-hover work.
MenuButton visibility/placement, checked split composition and shared native hover
are now connected. Full lifecycle, physical and installed-consumer runtime acceptance
remain pending.
