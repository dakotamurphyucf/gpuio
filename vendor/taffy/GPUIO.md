# Pinned Taffy adaptations

This is the unchanged-version Taffy 0.13.0 crate used by the pinned GPUI revision,
with scoped patches listed in `UPSTREAM.json`, which records the original
crate checksum, every archive file's SHA256 and upstream revision. The MIT license
is copied from that exact revision because the published crate omits it.

## Context retirement

`TaffyTree::clear` and `remove` retired nodes but left their entries in
`node_context_data`. GPUI stores text measurement closures there and clears its
layout tree each frame. Changing the shape of later frames could therefore keep
old text alive until the corresponding secondary-map slot was reused or the
whole window was dropped. This was reproduced independently with `Rc`/`Weak`
contexts: both operations left a strong reference with zero live nodes.

The patch clears/removes context entries alongside their nodes. It changes no
layout calculation or public API. Regression tests in
`rust/native/tests/layout_context_retirement.rs` cover immediate release, child
survival after parent removal and shrinking/reused trees. The native table
history test additionally exercises real GPUI text measurement and window cleanup.

Both the default backend and generated static-extension backends apply this
patch set and track this directory as a Dune source dependency. Keep those paths and
lockfiles consistent. Do not edit Cargo's registry cache or other projects.

To rebase or remove the patch, require the independent ownership regressions,
full native table traversal/revisit, normal layout/input regressions and both
backend builds to pass against the replacement dependency. Compilation alone
does not establish that retired measurement payloads are released.

## Intrinsic flex sizing

`intrinsic-shrink-factor.patch` corrects the reconstruction of a negative
intrinsic flex contribution. The calculation divides by
`max(1, flex_shrink * inner_flex_basis)` but previously multiplied by
`max(1, flex_shrink) * inner_flex_basis`. For fixed children with zero shrink and
negative margins, this amplified the margin by the child size, so three
overlapping 48px avatars could contribute zero width to their parent. The patch
uses the same factor in both directions; it does not change flexible free-space
distribution, public types, the pinned version or GPUIO's avatar API.

`rust/native/tests/flex_intrinsic_size.rs` checks 320 combinations of axes,
intrinsic constraints, sizes, shrink factors and signed margins, including
subpixel sizes. The production-host avatar regression checks the separate
overflow slot's actual painted position. Public-gallery and broader layout
validation are recorded separately; passing an engine unit test is not desktop
acceptance. Preserve both regressions when rebasing or removing this patch.
