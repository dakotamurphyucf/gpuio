# Measured carousel controls and card semantics — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This checkpoint connects the public constructor's default controls to the native
viewport and adds card set metadata. Offscreen focus/reveal, complete accessibility
qualification, gallery/public-driver and physical/resource/release acceptance
remain unfinished. No physical desktop window was opened.

## Contract

The paired protocol appends Kind55 `CarouselTrackGroup`; existing tags and the
unpublished epoch 3 remain unchanged. Core's existing outer column becomes this
explicit group. It has one CarouselTrack child and an optional plain Container of
up to 128 direct Buttons, with no implicit text or handlers on the group/container.
Selection and Press callbacks remain application-owned. Native code never infers
the relationship from a user key or label.

Candidate admission checks the shape atomically, including nonstructural changes
such as text and handler updates on a controls container. Review found that putting
all checks in structural validation would miss these edits; changed-node validation
now includes unchanged group parents for direct-child updates.

A valid pointer activation focuses the eligible viewport. Keyboard and semantic
activation preserve control focus. Home/End and axis arrows work from related
controls; nested editors and unrelated controls retain their own input. Native
routing rechecks the current relationship, admitted source and focus permission.
Removing the group removes this routing while retaining viewport/button owners.

Group hover and related-control focus pause automatic advance. Resuming starts a
full interval. Cards expose Group semantics, existing labels, one-based positions
and the full set size. The viewport remains a labeled Region.

## Tests and limits

Two production-host GPUI TestPlatform tests cover both-axis card semantics,
pointer/keyboard/AX activation with one Press, focus transfer/preservation,
control-arrow navigation, detaching the relationship, and automatic-clock pauses
for external hover/focus followed by a complete resumed interval. Native keyboard
activation requires key-down and key-up; semantic actions are asynchronously
delivered. The tests drive these mechanisms rather than bypassing them.

An admission test rejects malformed groups and child-only updates without changing
revision or retained bytes, and permits removing controls while keeping the
viewport. Core construction asserts the explicit group; Rust and OCaml independently
assert the appended kind byte, and Rust decodes the new Create envelope.

These are simulated-platform adapter tests. They do not qualify physical keyboard,
trackpad, IME, VoiceOver, hardware GPU or Linux desktop behavior. Offscreen focus
and reveal remain separate component acceptance work.

## Validation

Use `GPUIO_JOBS=2` and the isolated repository environment:

```sh
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test carousel_track
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: **700 native library tests/two existing private-D-Bus skips**, **339 protocol
tests/no skips**, five transport/admission tests, strict all-target Rust lint,
full OCaml tests/format/gallery build, catalog audit and whitespace checks.
A fresh installed gallery consumer builds successfully (`run=False`). An independent
installed Core/Bonsai probe checks construction, emitted group kind, custom motion,
layout callback reduction, selected-item changes and rejection after reconciler
close; it reports `GPUIO_INSTALLED_TRACK_FOCUS_PASS`. No installed physical app was
launched. OCH-41/OCH-17 remain open.
