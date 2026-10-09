# Sidebar destination styling

`Sidebar.Decoration.create` accepts optional `style` and `label_style`.
Both default to `Style.empty`. The destination style combines ordinary defaults,
`Appearance.item_style`, the per-item decoration style, then current-destination
defaults and `Appearance.current_style`. The label style belongs to the passive
text child. It does not restyle an icon, suffix, disclosure caret or tooltip.

`Sidebar.view` checks label styles against the composed-link passive contract:
no selectable text, scrolling, inert/disabled subtree policy or pointer shielding.
Normal highlight styles, including a Disabled appearance, remain available.
The link owns native focus, keyboard/pointer activation and the full accessible
destination name. Its passive label does not own a separate action or focus target.
Application state and the existing request reducer still own selection/expansion.

Every destination uses the existing composed `View.link` contract with keyed
icon and text slots. Adding, changing or removing a label style retains that
native destination and focus handle. Compact mode retains the full accessible
name and tooltip: it displays the icon when supplied, otherwise `compact_label`.
The unused text/icon slot is hidden so it does not create a spurious layout gap.
Disabled/re-enabled links retain their owner but rotate event handlers as required
by the existing reconciler fence. Unmounting retires all descendant resources.

## Name validation refinement

The experimental API now rejects blank destination names in `Item.create`.
The test is Core's ASCII-whitespace strip policy, matching `Link.Config`; the
existing valid UTF-8, no-NUL and 4,096-byte bound remains. Nonblank Unicode names
are preserved byte-for-byte. Compact fallback names keep their prior contract.
Previously whitespace-only labels could survive the item constructor; callers
must now supply a meaningful name. This avoids deferred link-construction errors
and does not truncate long names to the smaller rich-button label bound.

No new protocol operation, ownership mechanism, synchronous native callback or
timer is introduced. Context-menu wrapper replacement retains its existing
documented replacement behavior; ordinary style/collapse/selection changes do
not replace the destination.

The Journeys gallery exposes **Style sidebar labels**, with per-item rounding,
a highlighted Projects label and an independent count suffix. Toggle styling and
compact/offcanvas modes on the same retained sidebar.
