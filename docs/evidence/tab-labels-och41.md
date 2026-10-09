# Decorative tab labels — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
Core/Bonsai `View.tab_bar_with_labels` adds partial decorative label overrides by
stable `Choice.Id`. Configured names, selection and disabled state remain the
source of native semantics. Missing overrides fall back to string labels;
unknown/duplicate IDs and interactive or oversized labels are rejected.

The existing radio-slot mechanism supplies stable Container owners. TabBar now
admits the same bounded slots and revalidates late descendant changes atomically.
The renderer excludes those slots from generic child traversal so content paints
once. Simple/rich transitions retain the native compound focus owner and option
semantic IDs. No wire tag or separate selection owner is added.

The Navigation gallery's “Decorated workspace tabs” toggle switches badges on and
off while its editor panels retain state. It uses only public API constructors.
This is one part of [rich-tab parity](../design/rich-tabs.md); interactive suffixes,
five variants, per-tab target styling, overflow/reveal and native indicator motion
remain explicit required work.

## Behavior evidence

The Core expect test now covers both radio and tab constructors: rejected
unknown/duplicate IDs and nested actions, partial fallback, reorder without node
replacement, latest callback routing and label retirement. Native admission covers
both families' slot counts, final option configuration, late handler/style changes,
atomic rollback, reorder, fallback and disposal.

A production-host TestPlatform regression covers configured tab names, arrow
navigation skipping a disabled choice, accepted selection, stable semantic IDs
through reorder, semantic activation, return to plain labels and teardown. The
existing rich-label host fixture now includes tabs and verifies single paint,
single activation, per-option disabled appearance reaching avatar fallback,
retained native focus, decorative spinner activity and complete disposal. Initial
failures identified the missing TabBar child admission; a separate fixture error
used noncontiguous node allocations and was corrected without relaxing admission.

These are simulated-platform native tests. They do not prove physical macOS
keyboard, VoiceOver or hardware pixels; no OS window was opened.

## Validation

Use `GPUIO_JOBS=2` and the repository's isolated environment:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib control_labels_view_test
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test control_labels
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed so far: **706 native tests/two existing private-D-Bus skips**, four label
transport/admission tests, strict all-target lint and the targeted Core expect
suite. Full OCaml tests/format/gallery build pass. A fresh installed-package gallery build also passes with
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. Catalog and
whitespace audits pass. The physical gallery driver now includes rich/plain toggle
and retained-editor checks; its syntax passes, but the desktop run is unexecuted.
OCH-41 and milestone 07 remain open.
