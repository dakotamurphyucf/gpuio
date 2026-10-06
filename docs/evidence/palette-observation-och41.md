# Palette snapshot qualification — OCH-41

Local macOS 14.5 arm64 qualification, 2026-10-05. Base `5ea77dd` plus the
archived source changes. OCH-41 and OCH-17 remain open.

Core/Bonsai `View.command_palette ~on_change` now delivers native query,
composition, highlighted command and matched-count snapshots. The
[contract](../design/palette-observation.md) defines subscription ownership,
bounded delivery, query identity and the limits of application-side stale-result
checks. Native editing/navigation/command routing remains independent of OCaml
callback delivery. This does not yet add programmatic query/highlight commands,
atomic external-result admission, loading or persistent embedding.

## Passed checks

- Full native library suite with `native-image-tests`: **960 passed, two existing
  skips**. New TestPlatform cases cover initial/edge snapshots, no events from
  ordinary paint/focus, native arrow selection, query changes and composition,
  detach/reattach without replacing the query, registry-projection changes,
  dismissal, combined edit notifications and bounded ordered mailbox delivery.
- Paired OCaml/Rust fixtures independently assert operation 127 and event 79 bytes.
  Tests reject truncated/invalid envelopes and out-of-range state. Core expect
  tests cover latest callback dispatch, repeated/out-of-order snapshots,
  prepared-versus-accepted subscription changes, fresh handler generations,
  unmount and invalid future tree revisions. Query identity distinguishes
  selection-only changes from edits and observer lifetimes.
- Full OCaml `@runtest`, `@fmt` and gallery build pass. Strict Clippy passes for
  protocol/native all targets with `native-image-tests`; Rust formatting, Python
  driver compilation and whitespace checks pass. Two lint-only redundant
  conversions were removed after the native suite; strict Clippy recompiles them.
- A fresh independent installed gallery consumer builds and passes its catalog
  checks. The full macOS Feedback walkthrough passes: real Down/Up changes the
  OCaml-observed highlight; query filtering reports one result; real `z` typing
  reports none; Backspace restores one through a new native query revision.
  Existing menus, command activation, native focus/Tab order, retained header
  editing, rich/empty content, search policies, dismissal and notifications pass.
  The resulting palette screenshot was inspected.

Installed gallery SHA-256:
`057501a21aae024f0cbdf86e4e756aea8c3fae75f4949af5a7d4edecceef0ee3`.
The structural catalog audit passes; it is inventory evidence, not whole-family
behavior acceptance.

The GUI wrapper reaps its driver/application and restores the clipboard
representations saved before the run. No VoiceOver or OS preferences were changed.
TestPlatform composition is not real OS IME qualification; these results do not
establish Linux GUI or consolidated accessibility/performance acceptance.

## Reproduced issue and retained failures

A regression with two edits in one native update fails when query identity is
derived only from final text/composition: GPUI coalesces notifications, so changing
`Run` → `Other` → `Run` can reuse an obsolete identity. The implementation now
also observes the input's existing accepted-edit revision. The counterfactual
failure and corrected passing test are retained. Accepted replacements can
advance identity even when final text is unchanged; caret/focus/paint do not.

Earlier validation failures include a decoder helper typo, misplaced interface
documentation, two redundant Rust conversions and a test fixture that removed
registry entries while retaining palette references. The fixture now changes
the palette projection to empty without violating registry admission. An initial
Rust test command selected `native-tests`, which ran only two list tests; the
qualified full run uses `native-image-tests`. An incorrect direct Dune runner
path was replaced with the normal test alias. No acceptance assertion was removed.

[Commands, source changes, binary hash, original failures, passing logs and screenshots](palette-observation-och41/validation.tar.gz)
are retained with a [verified checksum manifest](palette-observation-och41/manifest.json).
Hosted run 37387307992 tests earlier `f6e34e2`; it does not qualify this source.
Current-source hosted checks/review and the remaining catalog/release gates are
still required.
