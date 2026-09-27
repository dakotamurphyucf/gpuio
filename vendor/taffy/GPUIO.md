# Pinned Taffy context retirement

This is the unchanged-version Taffy 0.13.0 crate used by the pinned GPUI revision,
with the two-line `context-retirement.patch`. `UPSTREAM.json` records the original
crate checksum, every archive file's SHA256 and upstream revision. The MIT license
is copied from that exact revision because the published crate omits it.

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
patch and track this directory as a Dune source dependency. Keep those paths and
lockfiles consistent. Do not edit Cargo's registry cache or other projects.

To rebase or remove the patch, require the independent ownership regressions,
full native table traversal/revisit, normal layout/input regressions and both
backend builds to pass against the replacement dependency. Compilation alone
does not establish that retired measurement payloads are released.
