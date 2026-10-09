# Cubic easing presets — OCH-41

2026-10-05, macOS 14.5 arm64 / Apple M1 Max. Implementation `8f9ab1b` adds
`Animation.Easing.ease_in_cubic` and `ease_out_cubic`, plus selectable examples in
the Motion gallery. They implement `t^3` and `1 - (1 - t)^3`; the existing CSS
ease-in/out presets have different curves.

Both polynomials use the existing native Bezier representation: x control points
1/3 and 2/3 make x linear, while y controls are both 0 for ease-in and both 1 for
ease-out. This introduces no wire tag, dependency or native production change.
The native timing/lifecycle/ownership path remains unchanged. Piecewise cubic
ease-in-out, step and linear-stop easings are not added by this change.

Validation completed locally:

- OCaml public constructors encode to the independently constructed
  `animation-cubic-easing.tsv` fixture; the existing view API expect suite,
  gallery build and formatting pass.
- Rust encoding agrees with the same fixture. Native easing evaluation matches
  each independently calculated polynomial at 1,001 points with error below
  `2e-12`, including endpoint and out-of-domain clamping checks. Both are checked
  to differ from CSS presets. All five animation protocol tests and strict
  targeted Clippy pass.
- The full real-window Motion gallery walkthrough passes: both new presets
  produce intermediate widths and exact endpoints in both directions; existing
  interruption, spring-sequence pause/resume/cancel/reverse, reduced motion,
  cross-window policy, shared phase, departure/remount and shutdown checks pass.
  Controls use native AX activation; the numeric test, not wall-clock AX sampling,
  establishes the polynomial curve. The captured layout was visually inspected.
- A fresh staged-prefix installation builds the independent gallery consumer and
  passes both extension catalog checks. It uses no dependency installation into
  an unrelated switch. This consumer was **built, not executed**; the native
  walkthrough above used the repository-built gallery.

Exact commands, terminal session results, both executable hashes, fixture and
logs/capture are in the [nine-file evidence archive](cubic-easing-och41/local-validation.tar.gz)
with a [verified manifest](cubic-easing-och41/manifest.json).

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest examples/gallery/main.exe @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol --test animation
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-protocol --test animation -- -D warnings
python3 scripts/test_gallery.py --section motion --executable _build/default/examples/gallery/main.exe --images <fresh-directory>
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-workspace>
```

Python compilation, catalog structural audit and whitespace checks also pass.
All owned local processes/windows have exited. These results do not establish
VoiceOver, Linux desktop, clean-machine packaging or physical presentation
latency acceptance. The [motion source review](../catalog/motion-review.md)
records the remaining differences; OCH-41 and OCH-17 remain open.
