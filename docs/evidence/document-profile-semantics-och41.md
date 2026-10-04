# Mounted document profile semantics — OCH-41

Checkpoint 2026-10-04, macOS arm64. Sources are the dirty worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`; HEAD alone does not identify them.
This is production-host behavior on GPUI TestPlatform, without OS windows.
OCH-41 and OCH-17 remain open.

## Behavior checked

The installed test profile now exercises `Text`, `NonText` and `Opaque` for both
an inline object and a block object. One mounted document transitions through
NonText → Text → Opaque → Text → NonText while its source revision stays fixed.
The new regression verifies:

- Declared Text participates in the production HighlightScope worker results and
  actually painted search backgrounds. Both custom objects match exactly once.
- NonText controls have no document-search glyphs. Opaque content produces
  `SourceUnavailable`, preserving the contract that partial counts must not be
  presented as complete. Old Text backgrounds disappear after either transition.
- Text uses native label/value semantics and suppresses the custom button
  renderer; NonText/Opaque retain labelled native button semantics.
- Select-all plain copy includes the plugin's declared copy text, independently
  of search eligibility. Source-format copy preserves the original Markdown.
  The window selection provider agrees with the document copy provider.
- Changing copy format preserves the installed displayed-text projection and
  selection, without changing source revision or reparsing the profile.

Only the native test fixture and regression module changed. No production,
protocol, public API, dependency pin or maintained vendor patch changed here.

## Validation

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`. Logs live under ignored
`scratch/agents/root-20261003-release-notices/`, not as build dependencies.

- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2 profiles::semantics::`:
  one test passed, covering all five installed transitions;
  `document-profile-semantics-native-002.log`. The first compile used an absent
  `StaticText` role name; the fixture now asserts the pinned reader's actual
  `Label` role and value, without weakening the semantics assertion.
- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2`:
  **915 passed, two existing macOS private-bus skips**;
  `document-profile-semantics-full-001.log`.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`:
  passed; `document-profile-semantics-clippy-001.log`.
- The structural catalog audit (146 modules / 43 families), edited-document
  relative links and `git diff --check` pass.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt`: passed;
  `document-profile-semantics-format-check-002.log`. The first check found only
  line wrapping after the AX role-name correction; that formatting was applied.

The existing `block 0.1.6` future-compatibility warning remains. The immediately
preceding [editor range API checkpoint](editor-range-api-och41.md) passes all 392
protocol tests and the full OCaml/gallery build. This later slice changes only
Rust tests and documentation; it does not claim a new OCaml or protocol run.

## Limits and remaining work

This checkpoint does not simulate pointer-drag partial selection; the subsequent
[selection repair](document-profile-selection-och41.md) adds that coverage and
96-passive-candidate focus traversal. Neither checkpoint proves physical clipboard
shortcuts, VoiceOver, OS accessibility hit-testing or platform shaping. Native
controls retain the earlier [event/focus evidence](document-profile-renderers-och41.md)
and [virtual focus evidence](document-virtual-focus-och41.md); arbitrary plugin-owned scroll containers still need qualification. Copy labels alone do not make a plugin searchable or prove
that an arbitrary custom renderer implements selection. Trusted plugin authors
remain responsible for truthful declarations and native control semantics.

The full macOS gallery/input/accessibility/performance/distribution gates and
required Linux nongraphical validation remain open.
