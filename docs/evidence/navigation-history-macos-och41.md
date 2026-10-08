# Navigation history gallery and macOS walkthrough — OCH-41

At base `98f652fa`, the gallery's history example demonstrated only back/forward
movement through three seeded entries. The new
[Journey preview](../../examples/gallery/journey_preview.md) separates its pure
model, Bonsai reducer and native view from the carousel/sidebar parent, then
exposes the existing public API's new visits, replacement, root/reset, Slide/Fade/
Immediate motion and Retain/Unmount policies. Repeated visits allocate distinct
instance IDs. Rejected history changes retain the model and display their error.
The root alone owns the note editor; revisiting the same route does not reuse it.
No library, protocol or native renderer implementation changes in this checkpoint.

The retained sidebar now has a destination-content panel, preserving a meaningful
workspace preview after extracting history into its own card. The adjacent
walkthroughs explain instance identity, effect execution, native editor lifetime
and the independent sidebar/history models. The new source/interface pair is
linked from the gallery README and included in the reviewed example inventory.

## Actual macOS results

macOS 14.5 (23F79), arm64. The repository executable and a freshly staged, separately
linked public-library consumer both pass six Dark/Light × Slide/Fade/Immediate
cases. Each case verifies:

- Disabled back/root/replacement controls at the root and disabled forward at end.
- Actual foreground Space activation of Continue journey and native AX actions for
  the other controls, with exact current chapter and back/forward counts.
- A native root draft surviving retained forward/back/root navigation while the
  hidden editor is absent from accessibility.
- A new visit discarding the old forward branch, replacement preserving the root,
  and a second Imagine visit being distinct from the root editor owner.
- Unmount policy retiring the hidden editor so returning seeds its original text.
- Reset creating fresh history; leaving/re-entering the entire gallery page also
  restores its initial state.

Both binaries close normally. The existing `--section journeys` walkthrough also
passes on the final repository executable, covering carousel draft/axis behavior,
history and sidebar selection/collapse. Two owned-window screenshots per six-case
run are retained. Final screenshots were reviewed for the navigation panel's
layout; they do not measure intermediate animation frames.

An initial six-case run passed before the destination-content layout adjustment.
Its sources differ in the parent layout and screenshot framing, and its report is
retained separately as `journey-native-001`. Final repository evidence is
`journey-native-002`; installed evidence is `journey-installed-001`.

## Commands and provenance

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @fmt
python3 scripts/test_gallery.py --section journey-history --images scratch/journey-history
python3 scripts/test_gallery.py --section journeys --images scratch/journey-existing
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/journey-consumer
python3 scripts/test_gallery.py --section journey-history --executable scratch/journey-consumer/consumer/_build/default/main.exe --images scratch/journey-installed
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
python3 -m py_compile scripts/gallery_journey.py scripts/test_gallery.py
ruff check scripts/gallery_journey.py
git diff --check
```

All pass. Local native runs use a 180-second exception/cleanup wrapper. The fresh
consumer validates both linked schemas without a window before desktop testing.
Actionlint passes for the new three-minute Foundation step. No new core/Rust tests
are claimed for this example-only change.

The [raw archive](navigation-history-macos-och41/reports.tar.gz),
[verified manifest](navigation-history-macos-och41/manifest.json) and
[binary/case summary](navigation-history-macos-och41/summary.json) retain reports,
logs, screenshots and exact final fixture/application sources. The initial layout
trial is not represented as the final source. Hosted/current Linux checks, scaled
geometry, VoiceOver, every-frame motion, resource bounds and consolidated OCH-41/
OCH-17 acceptance remain open. This does not resolve the loaded-list scrolling
report or establish Linux desktop qualification.
