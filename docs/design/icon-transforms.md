# SVG icon transformations

OCH-41, 2026-10-07. Interface drafted before implementation. The public API and
native paths are implemented; scoped validation is recorded separately in the feature evidence.
This contract does not certify the broader catalog or release.

The pinned Icon's transformation consists of independent x/y scaling, clockwise
rotation and logical-pixel translation. It does not expose a general shear
matrix. GPUIO exposes the same operations as an immutable `Icon.Transform.t`,
accepted by `Icon.Config` and `Icon.Decoration`. A later transform replaces the
whole previous transform; composition is explicit in the value's fields.

`Transform.create` defaults to scale (1,1), rotation 0 degrees and translation
(0,0). All inputs are finite. Scale components are in [-64,64], rotation in
[-360,360] degrees, and translations in [-16384,16384] logical pixels. Negative
scale mirrors; zero scale collapses artwork. `rotate_degrees` is the rotation-only
convenience. `Config.with_transform t None` restores the default presentation.
These explicit resource/numeric bounds differ from unchecked Rust builders.

Transformation is visual: scale about the assigned icon box center, then rotate
clockwise, then translate. It does not change layout size, intrinsic dimensions,
the enclosing button's hit region, accessibility geometry or focus owner. Parents
still clip artwork according to normal overflow rules; native OS menus retain
their bounded 16-logical-pixel icon slot. Existing fit, foreground/alpha, rounded
image clipping, meaningful/decorative semantics and source lifetime must survive.
Transforms on decorations must reach button, tab, menu and other existing slots.

An additive `Set_icon_transform` operation (tag 128) carries an optional five-float
record; matching OCaml/native packages are required. Native admission validates
values and Icon kind atomically. Omission
defaults to no transform; reset is explicit when removing a previous transform.
Transform changes must not recreate source registrations, change handlers or
reset native input. No timer or asynchronous OCaml layout/paint callback is added.

The native renderer reuses decoded alpha and the existing GPU mask affine
path; position/angle/color changes do not rerasterize the SVG.
Fit/size-dependent raster work remains in bounded workers. Rounded clipping is
applied to the worker mask before the visual transform.
Icon overflow defaults to visible; explicit own/ancestor overflow clips after
the transform. A fit, size, density or corner change queues bounded worker work
and may display the previous completed mask until the replacement is ready.
A color or transform change uses the current mask immediately.
Native AppKit popup icons use bounded template snapshots, so their transformation
must be applied when the snapshot is created and clipped to its existing slot.
An already tracking menu follows the existing snapshot/cancellation contract;
it is not a continuously repainted GPUI surface. Do not silently ignore transforms
on that path. Validate actual platform output independently from GPU output.

Required qualification includes independent paired bytes and malformed values;
Core config/decoration/reconciliation update/reset behavior; atomic wrong-kind
and invalid-value rejection; asymmetric SVG GPU positive/negative pixel checks
for scale/rotation/translation/reflection, fit, clipping, themes and densities;
unchanged control input/AX identity; native menu snapshots; retained-source/cache
accounting and disposal; and a public gallery with adjacent beginner walkthrough,
including a fresh installed consumer. Linux build/unit/consumer checks remain
required, with full Linux desktop qualification deferred under OCH-47.
