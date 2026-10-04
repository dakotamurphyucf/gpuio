# Pinned GPUI core adaptation

GPUIO uses GPUI 0.2.2 from Zed commit
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`, unchanged from the original project pin.
`vendor/gpui` contains that crate and its Apache-2.0 license. Other Zed crates
remain dependencies of the same pinned Git revision. The repository and generated
application workspaces patch that Git source to the same local GPUI crate, keeping
one native GPUI type universe. Applications using custom Cargo manifests must
preserve the generated patch alongside the existing crates.io patches.

`third_party/sources.json` records the archive and patch SHA256 values.
`scripts/vendor_gpui.py` verifies both before reconstructing a fresh destination.
It expands inherited dependency declarations from the pinned Zed workspace,
including feature unions and aliases, converts workspace paths to Git dependencies
at the same revision, retains `Cargo.toml.upstream`, copies the actual license
instead of its upstream symlink, and declares the known `rust_analyzer` cfg for
the standalone manifest. It does not mutate shared Cargo sources or toolchains.
`third_party/patches/gpui.patch` is the complete source-code difference.

## Hidden pressed elements

The pinned `Interactivity::paint` returned early for hidden visibility, before
registering its active-state release handler. A pressed style that hid its own
element therefore persisted after mouse-up. Display-none elements also need
cleanup despite having no visible hitbox. Retained pending activation could
otherwise be replayed on a later visible frame.

The patch cancels pending mouse/keyboard activation when the element is hidden
or display-none. For an existing active state it registers only capture-phase
cleanup: mouse-up, or a later mouse move reporting no pressed button, clears the
state and refreshes the window. It does not register hidden hit/click handlers,
emit an application click, reset unrelated focus/scroll state, or change IDs.
Native tests verify repeated inside/outside releases, lost-release motion,
Visibility/Display recovery, cancellation of stale activation and a fresh visible
click. This is not a claim that every ancestor-removal or platform input case has
been exhaustively validated.

## Active-only hitboxes

Pinned GPUI's `should_insert_hitbox` considered hover, focus tracking and input
listeners but omitted `active_style` and `group_active_style`. A passive document
wrapper with only a pressed style therefore had no hitbox, so the standard active
state handlers could not observe its press. The adaptation includes both active
style declarations in that admission test. It preserves ordinary hit testing and
event propagation, without adding a synthetic click listener or focus handle.
The document visibility suite exercises a pressed-only wrapper, its hidden state,
and release outside the wrapper in source and Markdown presentations.

Before rebasing/removing the patch, rerun `native_highlight_view` with
`native-image-tests`, `native_highlight_document`, document and controls regressions,
library tests, and default
and independent generated-backend builds. Require visible restoration and queued
event evidence; successful compilation alone is insufficient. Reconstruct and
compare the vendored tree, review lockfile changes, and preserve the required
Linux build/unit/private-bus/consumer gates. macOS GUI evidence does not qualify
the deferred Linux desktop milestone.

## Disabled accessibility scopes

`Window::with_a11y_disabled` marks nodes produced during a prepaint scope as
disabled without introducing a wrapper identity. GPUIO's retained native tree
enforces input, focus and popup policy independently; this API only controls
accessibility output. This matters for type-erased `AnyElement` wrappers, whose
own accessibility identity is absent: attaching disabled metadata to such a
wrapper alone does not reach its actual rendered children.

The accessibility builder normalizes disabled inheritance in the completed
frame, clearing actions on disabled nodes and descendants while preserving their
roles, names, values and IDs. Disabled focus falls back to the window root.
Normalization works on output nodes, so re-enabling restores the original
actions and leaves independently disabled controls disabled. Nested prepaint
scopes restore their caller's state; a new frame resets the scope.

The pinned GPUI accessibility unit suite covers both scope-produced nodes and
ordinary/synthetic descendants, re-enabling, original-node preservation and
unrelated siblings. Native and public gallery acceptance is tracked in the
[disabled-subtree contract](disabled-subtrees.md). When upgrading this patch,
rerun those checks as well as the existing focus/active-descendant unit suite.
This is not a claim of VoiceOver or Linux desktop qualification.

## Window text-input context lifetime

The pinned macOS backend uses one NSView input context across native editor
owners. Removing a composing editor replaced its input handler but left the
OS conversion session alive: an actual US Option-E accent from the retired
Settings name field appeared in the next numeric field (`´1e-` instead of
`1e-`). Changing only the OCaml saved draft cannot repair this platform state.

The local GPUI adaptation attaches the existing focus identity to each painted
input handler and supplies an opt-in `Window::on_text_input_reset` adapter.
During frame handoff, an owner change unmarks the previous client before calling
the adapter. Ending a composition also calls it, including a platform composition
that starts and ends between paints. The adapter runs with no platform input
handler installed, so synchronous OS text-client callbacks cannot edit the next
owner or re-enter GPUI. Unmarking uses the old client's own text policy; this
does not impose a new universal commit/cancel policy on all editor types.

GPUIO installs a macOS adapter on every application window that invokes
`NSTextInputContext.discardMarkedText`. Apple requires the client marked range
to be cleared when discarding the conversion session.
[Apple documentation](https://developer.apple.com/documentation/appkit/nstextinputcontext/discardmarkedtext%28%29?language=objc).
The adapter neither changes the keyboard source nor activates the window.
Ordinary draws, layout/style/configuration changes and cached paint of the same
composing owner preserve its session. Other platforms do not install this hook;
their required build/unit checks remain separate from desktop qualification.

The GPUI input unit suite checks redraw/configuration retention, completed
composition, blur, removal of a painted input while its entity and focus handle
remain alive, and mark/unmark callbacks coalesced before a paint. It also checks
that no platform handler is installed during reset. The public Settings
`--section settings-composition` regression exercises real US dead-key input,
owner retirement and fresh typing after Escape. This does not qualify multilingual
candidate panels, VoiceOver or Linux IME behavior. Rerun these checks and the
native editor/numeric/OTP regressions before changing the adaptation.

## Read-only accumulated opacity

Custom text-shimmer paint needs the opacity actually computed by GPUI, including
ancestors, interaction states and animated styles. The patch makes the existing
`Window::element_opacity` getter public and corrects its documentation to allow
paint or prepaint, matching its existing phase assertion. It does not change
opacity calculation, accumulation or rendering. The shimmer adapter suppresses
its overlay and recurring wake when that value is zero. Copying only declared
node styles would miss inherited and state-specific opacity.

The updated patch and archive hashes reconstruct exactly to `vendor/gpui` with
`scripts/vendor_gpui.py --archive <verified archive> --output <new directory>`.
Mounted shimmer tests cover zero-opacity ancestors and restoration; this accessor
does not establish whole-application idle or Linux graphical acceptance.

## Decoded image alpha masks

The [custom spinner contract](custom-spinner.md) needs rotation of SVG artwork
already decoded by GPUIO's bounded worker pipeline. `Window::paint_image_mask`
extracts coverage from a decoded BGRA frame only on an atlas miss and uses GPUI's
existing transformed monochrome sprite. `AtlasKey::ImageMask` is distinct from
the ordinary color-image key. Color, inherited opacity and transformations reuse
the mask upload; `drop_image` retires color and mask entries for all frames.
No shader, SVG parser, asset loader or platform dependency is added.

Invalid frame indices, nonfinite geometry/color/transforms and overflowed coverage
return errors before atlas admission. Empty bounds, zero paint alpha/opacity and
fully clipped transformed bounds return without allocating a mask tile. GPUIO
must separately admit and account for mask representations before exposing this
through its public spinner; this low-level GPUI method supplies no GPUIO quota.

Scene culling and draw-order bounds now use transformed coverage for monochrome
and subpixel sprites. The previous code considered their untransformed quad,
which could omit an icon translated into a clip. Original quad/texture coordinates
and shader transforms are unchanged, with an identity fast path. TestAtlas now
records the key's real texture kind rather than labelling every upload monochrome.

Two asset unit tests pass in the isolated GPUI test crate. GPUIO's
`image_mask_test` checks scene/cache behavior without an OS window, and the full
native feature-enabled library suite passes 430 tests with two existing ignores.
These checks do **not** establish GPU pixels, physical presentation, performance
budgets or Linux graphical behavior. Real rotated-asymmetric-mask GPU tests and
complete public spinner/resource acceptance remain open.

The cumulative patch SHA-256 is
`ef2f001a5447fb86f78673bd9ce8bbda9ad9d20549ef60823b748d946a4bf866`.
Reconstruction with `scripts/vendor_gpui.py` from the hash-verified pinned archive
matches `vendor/gpui` exactly. Source pins and dependency versions are unchanged.
The full Rust workspace and a fresh installed-gallery consumer build also pass;
the latter stages public libraries into an isolated prefix and composes its own
native backend against the patched GPUI. It was not launched, and it is not
clean-machine distribution evidence. Commands are in the linked spinner contract.


## Explicit composite active-descendant ownership

The native picker renders its query input and list as siblings. GPUI's existing
ancestor-only active-descendant API ignores an option when that sibling input owns
focus. The additive `aria_active_descendant_for(&FocusHandle)` stores a weak owner
and checks both current keyboard focus and the accessibility tree's real focused
node before claiming accessibility focus for the option. It does not move keyboard
focus, retain a removed input or reparent the input/list. The input must already
have been exposed in that frame's prepaint; GPUIO orders query before list.

Missing, unfocused and self targets are ignored. Ordinary ancestor-based behavior
and the existing duplicate-claim safeguard remain. GPUIO declines explicit query
ownership during marked composition, preserving input accessibility focus then.

Seventeen isolated `window::a11y::tests` pass. Three new tests cover explicit
ownership with independent input value/ancestry, wrong/missing/self owners and
frame retirement, duplicate-claim protection, and the actual element prepaint
path on TestPlatform. The rendered fixture compares ordinary ancestor behavior,
explicit query ownership and focus on another control while checking unchanged
real keyboard focus. These create no OS window and do not establish external AX
notification delivery, VoiceOver or physical IME acceptance.

The cumulative patch SHA-256 is now
`b59b2178351ff990964ecf1746b5b739f496ca39aced0c31f8bf747490d79c6d`.
`third_party/sources.json` records it. Reconstruction from the existing verified
pinned archive with `scripts/vendor_gpui.py` matches `vendor/gpui` exactly; source
pins, dependency versions and licenses are unchanged. The integrated native and
consumer checks are recorded in the OCH-41 evidence ledger.

## Grouping mounted accessibility children

`A11ySubtreeBuilder::parent_id` exposes the real node identity to element wrappers.
`group_children` inserts a stable synthetic container around an ordered contiguous
range of existing direct children. It preserves their IDs, actions and subtrees,
replacing the range at its original position. Empty/repeated/noncontiguous/foreign
members, occupied synthetic IDs and containers with preexisting children are
rejected before mutation. No private element-ID hashing is reproduced in GPUIO.

The picker captures mounted row IDs and projection indices during prepaint, then
uses this helper to create named Group parents. It does not mount offscreen rows
or copy group text per option. The group identity is independent of visual header
visibility. This helper does not add rollback to GPUI's `Window::transact`; actual
autoscroll prepaint-retry accessibility behavior remains a separate validation gap.

TestWindow now retains accessibility initialization callbacks and the latest tree
update. VisualTestContext exposes activation/deactivation and tree inspection,
using normal platform callback paths without an OS window. This enabled actual
picker tree checks and exposed duplicate query rendering in GPUIO's generic child
loop, now repaired by mounting structural slots only through the picker renderer.
TestPlatform observations do not establish physical accessibility delivery.

The cumulative patch SHA-256 is
`53b0aac8cfa1823be60b7aab8e9c10b0792e0ea095bb8734f7054124ce037089`.
Reconstruction from the verified pinned archive matches `vendor/gpui` exactly.
Pins and licenses remain unchanged. Validation commands and limits are recorded
in the OCH-41 gallery evidence ledger.

## Axis-aware measured lists

`ListState::new_for_axis` and `with_uniform_item_extent` generalize the existing
measured-list engine without duplicating its sum tree, anchors, retained focus,
remeasurement or tail-following state. The original constructor remains vertical.
Private coordinate mapping treats cached width/x as cross-axis and height/y as
main-axis, converting at physical layout, child prepaint, content-mask,
autoscroll, wheel and public bounds/scrollbar boundaries. Text, child widgets,
hitboxes and accessibility nodes keep ordinary physical coordinates.

Initial extent hints now survive first layout. Cross-axis resize invalidates
measured items while retaining their previous extents as estimates; clearing
those hints collapses distant unknown items and breaks reliable distant reveals.
Visible measurement replaces estimates. Reported item bounds include leading
padding, and scrollbar range includes both main-axis pads. Perpendicular-only
wheel deltas do not rewrite the logical scroll anchor or emit list scroll work.

The horizontal public Core/bridge/Host API remains unfinished. The typed plan is
`docs/design/horizontal-managed-lists.md`; a native constructor alone is not a
shipped horizontal managed-list capability. Preserve that boundary in the catalog.

## Accessibility rollback during prepaint retries

A child autoscroll request can discard a list's first prepaint and retry at a new
offset. The pinned `Window::transact` rolled back layout/hitboxes/dispatch but left
accessibility nodes from the rejected pass, causing duplicate node IDs on retry.
It now checkpoints accessibility when active and commits or rolls back with the
same transaction result. Completed sibling subtrees are not copied: the checkpoint
retains the completed-node count, open ancestor nodes and focus/active-descendant
state; a mutation journal records focus-ID/bounds-map changes only during active
transactions. Rollback removes newly completed IDs/debug provenance, restores
ancestors and reverses those mutations. Nested successful transactions keep their
journal entries until the outer transaction finishes. Paint-time action listeners
are created only for the accepted prepaint.

Regression coverage belongs in `rust/native/src/horizontal_list_test.rs`: actual
native list prepaint retries with accessibility active, physical bounds, focused
children and prepend behavior run for both axes. This does not establish physical
VoiceOver speech or macOS GPU acceptance. Reconstruct the entire recorded patch
from the pinned archive when changing either adaptation.
