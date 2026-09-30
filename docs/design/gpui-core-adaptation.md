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
