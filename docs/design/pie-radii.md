# Pie radii — OCH-41

Implementation contract, 2026-10-06. [Scoped local qualification](../evidence/pie-radii-och41.md)
records completed checks and outstanding catalog/platform limits.

The pinned PieChart accepts global and per-arc inner/outer radius accessors.
GPUIO represents those results as bounded values computed in OCaml before view
submission. No Rust layout/paint callback enters OCaml. This covers variable-radius
pies and donut sectors without changing immutable source values or angular shares.
Outside captions, leader-line colors and label gaps remain separate catalog work.

Keep `Pie.create ~inner_radius` as the existing donut fraction in [0,0.95]. Add
`Pie.Radius.Fit | Pixels of float` for the global outer radius, default Fit.
Pixels is finite and in (0,32768] logical pixels. Fit preserves the current radius
of half the smaller available plot dimension, after native presentation gutters.

`Pie.Slice_radii.create ~slice ~inner ~outer ()` constructs an abstract ID-keyed
override. Both radii are logical pixels with finite
`0 <= inner <= outer <= 32768`. `Pie.create ~slice_radii` accepts at most 256
unique IDs, matching the dataset slice limit. Unknown IDs are retained but ignored;
caption text and source position are never identities. Overrides replace both
radii; unmatched slices retain global outer radius and fractional inner radius.

An equal inner/outer pair, including zero/zero, has no visible wedge or label.
Its positive source value still reserves its angular interval and remains in the
original-data companion. An absent wedge cannot be hit or keyboard-previewed;
native selection highlighting is retained only when its ID has a prepared mark,
as in the existing zero-weight behavior. A zero-valued source slice is
still invisible even with a positive radius override. Radii are presentation,
not an implicit change to weights or hidden data filtering. Large radii may clip
to the plot; the renderer must not move the center or silently change other slices.

Existing inside captions use the overridden ring dimensions. Hit geometry and
inspection must use the same wedge radii. Reordering, renaming, publishing or
resetting data preserves the ID association and original value provenance.

Options schema advances from 7 to 8 with explicit rejection of older frames.
Style/data/chart-view schemas remain unchanged. The paired decoder admits at
most 256 overrides and a 16 KiB standalone options frame, rejecting malformed
IDs, counts, numbers and trailing/truncated bytes. Retained chart configuration
accounting includes the radius vector. Matching OCaml/Rust packages remain
required; no compatibility with previous experimental bridge revisions is implied.

Before acceptance, require independently specified paired bytes; OCaml validation
and round trips; native geometry, hit/provenance and reorder/zero/invalid cases;
actual GPU pixels; and public gallery controls with root and installed-consumer
checks. Preserve the previous no-override geometry and record platform/source
scope explicitly. This does not complete whole-pie/catalog or release acceptance.
