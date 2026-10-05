# Rich avatar fallback

OCH-41. Implemented locally; native GPU/accessibility and public-gallery acceptance
are still pending. See the [avatar source review](../catalog/avatar-review.md).
This addition preserves `Avatar.Config` and the existing `View.avatar` leaf API.

## Public interface

```ocaml
val avatar_with_fallback
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Avatar.Config.t
  -> fallback:'action View.t
  -> 'action View.t Or_error.t
```

The Core constructor checks the fallback, and `Gpuio_bonsai.View` specializes it
to Bonsai effects. `Avatar_group.Item.create_with_fallback` is the checked group
item constructor. Keeping the view-dependent API in `View` avoids an
`Avatar`/`View` module cycle. No image decoder, I/O task or runtime is owned by the
group. Its shared sizes/overlap style the avatar root as before.

One retained child is the fallback slot. Its key controls normal reconciliation;
image readiness does not require an OCaml transaction to mount/unmount it. The
root retains identity across source, label, palette and fallback changes. The
root config supplies the accessible description; rich visual content is
semantically decorative even while visible. Text initials remain the compatible
leaf API; a rich slot replaces their normal fallback presentation.

## Admission and ownership

The slot admits passive layout, text/styled text, images/icons, avatars, loading
and animation. It shares the Link passive-content rules: at most 4096 nodes and
128 levels, no action/observation callbacks, selectable text, controls/editors,
scroll owners or pointer shields. Core validates construction; native transaction
validation rechecks the final subtree and dirty ancestors after descendant
style/config/handler changes. An avatar has zero or one child; ordinary images
remain leaves. Global tree/depth/resource budgets still apply independently.

The paired capability is bit 56 (`CAP_AVATAR_FALLBACK`). The full required mask
is `9223372036854775807` (2^63−1). Existing operation tags are unchanged: `SetAvatar`
retains its record and normal `Splice` attaches the slot. Independent Hello bytes
are `0001fc0000000000000001` for this bit and `0001fcffffffffffffff7f` for the full
mask. An older host must reject the new client's capability requirement.

Registrations stay application-owned. Native fallback nodes and acquired image
leases remain retained while the primary image is shown. Returning to fallback
reuses them even if the application has already released the source registration.
Removing the slot, limit eviction, unmount or window teardown retires native
owners normally. Decode completion may cause finite work; hiding the fallback
suspends its motion instead of keeping a recurring frame/timer alive.

`avatar_slot.rs` owns slot visibility independently from container queries. Query
synchronization cannot overwrite the avatar's hidden-child set. All fallback
nodes remain visited for retention; only the selected subtree is constructed at
frame preparation. Common ancestry visibility gates both tween timers and
animation programs. The end-of-frame pass hides unpainted slots and pauses their
motion. Native layout/paint never calls an OCaml render callback.

## Frame selection and styling

A GPUI container-query element supplies the avatar's assigned size without letting
the child size the avatar. During prepaint, native code resolves the primary:

1. Raster images use the normal GPUI `Img` path, including its animation lifecycle.
2. SVGs prepare at the measured size/density and freeze their image/status for
   paint. A size failure can select rich content in that first affected frame;
   recovery can select the primary in the first valid frame. Ordinary tinted
   icons still prepare during paint so hover/pressed foreground colors work.
3. Missing/loading/failed candidates build the retained rich subtree, centered
   in a full-size slot. The fallback semantic wrapper is hidden; its containing
   avatar owns the only accessible name.
4. Paint consumes the prepared candidate. A GPU upload failure first discovered
   during painting is a later failure boundary: the SVG path records it and
   requests a subsequent frame to choose fallback. Same-frame arbitrary-subtree
   recovery after GPU upload is not promised. Raster upload behavior remains the
   existing GPUI image path and needs native qualification.

The pinned Base `AvatarFallback` is a styled child container. Its styled-layer
`.rounded().overflow_hidden()` does **not** establish a rounded mask for arbitrary
descendants: GPUI's overflow mask is rectangular. GPUIO follows this distinction
explicitly. The root background and primary image have root corner radii,
including state refinements; rich children are centered and clipped to the slot
rectangle. A full-bleed child image or colored surface must carry its own corner
style. Centered icons/initials need no artificial inset. We do not shrink every
custom subtree into an inscribed square or claim rounded clipping that the
renderer cannot provide. The earlier draft's stronger general clipping proposal
is superseded by this source-backed placement contract.

The gallery's team preview registers an original person SVG in its existing
window scope. `Custom avatar fallback` switches to the checked item API; image,
invalid-image and no-source modes exercise native selection while preserving
member identity. Sizes, overlap, palette, rounded-square styles, limit and reorder
remain available. The public native driver includes this transition sequence;
it has not yet run successfully under the current desktop restrictions.

## Validation and limits

`avatar_slot_test.rs` runs the production retained View and image service on GPUI
TestPlatform, with both raster and SVG primaries. It verifies primary/fallback
switching, a released-but-retained fallback source, independent query visibility,
retained animation identity, cancelled hidden tween deadlines, no additional tween
frame requests after advancing the test scheduler, and resource retirement on
unmount. The fallback also contains a nested avatar with a delayed animation
program. Hiding the outer slot cancels both motion deadlines, keeps both owners
for restoration and causes no timer wake when advancing the test scheduler by
20 seconds. Removal releases weak references to both owners. A separate
TestPlatform close-window scenario retires the Session tree and removes the
GPUI window while both motion deadlines are live. It checks View/motion owner
release, empty motion declarations, encoded fallback lease retirement and no
later output after advancing the scheduler, before shutting down the global
image service. This establishes the tested lifecycle on TestPlatform, not native
macOS window-close behavior. An inert ancestor preserves fallback paint while pausing its clock;
a hidden ancestor suppresses paint, and restoration retains animation identity.
That inert case fails against the initial implementation that used focus
eligibility to decide whether to paint. SVG cases assert selection immediately after the first oversized frame
and first recovered frame. The separate `image_prepaint_test.rs` checks phase
ordering before paint and preserves ordinary icon paint-time behavior.

These tests create no OS windows and establish no GPU pixels, physical input,
macOS AX tree or animated-raster presentation result. Native group/slot geometry,
clipping over contrasting backdrops, hover/per-corner styles, real semantic
ownership, actual animation-program presentation, source
observers and window-close/resource acceptance still require validation.
A fresh installed-gallery consumer now builds against staged public libraries
without changing an opam switch (`run=False`). Actual consumer GUI execution and
required hosted Linux/macOS checks remain part of catalog/release acceptance. Do not mark the family complete from
this implementation or the TestPlatform evidence.

The earlier actual `native_images` attempt reported macOS desktop-service errors
and reached its watchdog without checks passing. No native acceptance is inferred
from that run, and no new GUI test was opened while implementing this addition.

### Native acceptance fixture (execution pending)

`avatar_rich_test.rs` runs after the existing gradient fixture in `native_images`,
reusing its background window and sequential node slots. It contains assertions
for actual GPU pixels and native macOS semantic objects:

- A 96-pixel child in a 64-pixel avatar over a contrasting backdrop: rectangular
  overflow clipping, child-owned individual corners and synthetic GPUI hover
  refinement/restoration.
- Labelled and decorative roots with no fallback label/image/initials exposed
  as a second semantic owner.
- A released source retained by the fallback image while raster/SVG primaries
  display, followed by recovery without registering the child source again.
- Invalid primary data selecting rich content; an animated GIF displaying both
  decoded frame colors; ordered root image observations with no child callbacks.
- Removing the animated primary and retained fallback, weak-owner retirement,
  zero retained tree/source bytes and stable render counts afterward.

The fixture has been authored and compile-checked, **not executed successfully**.
Its assertions are planned acceptance evidence, not results. The AX checks do
not establish VoiceOver behavior, the hover checks do not establish physical
pointer delivery, and mailbox observations do not establish OCaml effect delivery.
Native window-close and public group/consumer acceptance remain separate work.


## Local checks — 2026-10-01 UTC

On local macOS arm64, `83eb87e` plus these uncommitted changes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2 --no-fail-fast
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/avatar-rich-installed-consumer
```

All passed. The feature-enabled native library run reported 426 passing tests
and two ignored private-bus tests on macOS. Native transaction tests additionally
cover descendant mutation rollback, one-slot admission, the 4096-node budget and
independent capability encoding. Three Core expect tests cover passive admission,
4096-node/128-level boundaries, group identity/retirement, idle reconciliation and
independent Hello bytes. The final `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt`
also passed after the inert-ancestor correction. Rust formatting, Python/shell syntax and the
structural catalog audit also pass. These are local results; the new code is not
in the earlier hosted `e54d279` run and no Linux result is claimed for it yet.

## Native follow-up — 2026-10-05

The previously unrun rich-avatar fixture now [passes locally, together with the
fresh installed public-gallery walkthrough](../evidence/avatar-native-consumer-och41.md).
The dated evidence records real GPU clipping/child corners, primary/fallback
transitions, ownership, native semantics and cleanup, with scope limits. This
does not claim VoiceOver, Linux GUI or final distribution acceptance.
