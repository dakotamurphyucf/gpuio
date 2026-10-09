# Native motion programs

Read the [implementation walkthrough](main.md) for actual model/functions,
Bonsai syntax, native ownership and diagnostic boundaries. Source: [main.ml](main.ml);
dependencies: [dune](dune).

Run `./scripts/gpuio exec dune exec examples/animation_program/main.exe`.
Use Pause/Resume, Restart, Reverse and Cancel on the two-stage reveal. Add another
activity bar to see it join the existing shared phase. System reduced motion is
respected; the demo uses only public OCaml/Bonsai/Eio APIs.

For the reproducible macOS integration check, build normally and run
`_build/default/examples/animation_program/main.exe --self-test`. It opens a local
window, validates ordered native observations and closes itself. No network or
credentials are required. This exercises the bridge; the native test separately
checks actual geometry, scheduling and disposal. Linux desktop qualification is separate under the
[platform policy](../../docs/platform-release-policy.md).
