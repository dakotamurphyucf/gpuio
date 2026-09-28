# Pinned plotting source

`src/sankey.rs` is from Longbridge GPUI Kit, Apache-2.0:

- Commit: `84f57fdfcb4910623fb0bb7f795b077e249f9271`
- File: `crates/component/src/plot/shape/sankey.rs`
- URL: https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/component/src/plot/shape/sankey.rs
- Original SHA-256: `a89ad6f6af462e71e970c68e4f0044a6df1bc1eee0009734fd7c05e876b05a15`
- Local changes: import `origin_point` from this crate rather than `crate::plot`.
  Iterate mutable columns directly in left-to-right relaxation, preserving order
  while satisfying the pinned toolchain's `needless_range_loop` lint.
- `origin_point` is the corresponding small coordinate helper from upstream
  `crates/component/src/plot/mod.rs`.

The existing GPUI revision is retained. This isolates the reusable topology,
layout and ribbon geometry from the styled component crate, its theme/macros,
and its different GPUI package pin. Upstream tests are retained.

This low-level source expects valid finite input and exposes layout internals;
it is not the validated public OCaml data/resource contract. GPUIO's chart
adapter must validate input again after decoding, bound graph/iteration work,
normalize large raw flow magnitudes before f32 geometry, retain original values
for semantic events, and cache layouts outside per-frame painting. Source
compilation/unit tests do not establish native chart interaction acceptance.
