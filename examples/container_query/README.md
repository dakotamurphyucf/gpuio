# Responsive workspace

Read the [implementation walkthrough](main.md) for actual model/functions,
Bonsai syntax, native ownership and diagnostic boundaries. Source: [main.ml](main.ml);
dependencies: [dune](dune).

Run `./scripts/gpuio exec dune exec examples/container_query/main.exe`.
Resize across 600 logical pixels to switch between compact and wide presentations.
Each presentation has its own retained editor and Bonsai counter. The native
layout chooses immediately; the observation callback never returns a view.

Both Bonsai computations remain active while one presentation is hidden. Hidden
editors preserve their drafts but cannot receive focus/input. Use application
state shared outside the branches when two presentations should share a model.

`_build/default/examples/container_query/main.exe --self-test` opens a local
window, resizes it, verifies hidden-editor state/focus denial and Bonsai lifecycle,
and closes it. No external services are needed. Native tests separately exercise
actual button dispatch and assigned-size geometry. Dated macOS evidence is recorded in
[container-query evidence](../../docs/evidence/container-queries-och26.md).
Current Linux desktop scope follows the
[platform policy](../../docs/platform-release-policy.md), with OCH-47 deferred.

On macOS this self-test requests focus so the window actually paints; an occluded
background window may accept the tree and initialize editors without painting.
