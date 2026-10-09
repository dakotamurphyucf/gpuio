# Pagination chooser evidence — OCH-41

## Per-gap popup ownership follow-up

The managed chooser now composes a persistent `Navigation.Gap_popup` around each
native gap button. Core tests check invalid passive-popup combinations, bounded
factory calls, retained native identities, disabled content suppression and no
compact-layout construction. The six exact public transactions cover closed,
first gap open/closed, second gap open, replacement by the first and final close.
A captured dismissal from the replaced opening cannot close its successor.

Native TestPlatform replay checks each button's stable AX identity, dialog-popup
kind and independent expanded state, placement bounds matching that gap (the
probe excludes its one-pixel border), one numeric owner across replacement and
focus return to the correct gap. The previous whole-row anchor is removed; no
new wire operation was needed. Fixture IDs/hierarchy changed deliberately.

Replacement exposed a real previous-focus edge: a new scope could remember the
retiring numeric field instead of its button. Direct-button popovers now retain
the declared trigger ID as their preferred eligible return target. A separate
native regression covers semantic activation without focusing the trigger first,
disabled/inert/removed triggers, custom anchors and focus moved outside before
closure. It verifies previous-focus fallback and no unwanted focus stealing.

Local scoped OCaml checks and native replay pass. Full native library checks
pass **631 tests with two existing private-D-Bus skips**. Full OCaml `@runtest @fmt examples/gallery/main.exe`, strict all-target native/
protocol Clippy, Rust formatting, catalog source audit and diff whitespace checks
also pass. These are
deterministic host tests, not real desktop/VoiceOver/IME or GPU qualification.

The authored macOS gallery walkthrough now uses the current 120-page example
and checks independent gap expanded state, semantic opening/focus return, real
numeric keyboard entry, explicit confirmation and subsequent paging. Its Python
syntax passes; this expanded desktop scenario has not been executed.

2026-10-02, macOS local dirty worktree based on `83eb87e`. This is a local
implementation checkpoint, not OCH-41/OCH-17 release completion.

## Covered behavior

Three added Core expect tests verify compact layout accessible labels and retained
button identity, disabled/empty boundaries, bounded formatting, interactive gap
ranges/callback refresh and stale-handler fencing, and passive breadcrumb members
without losing unrelated route actions. The existing navigation/model suite also
runs. `@test/view_api/runtest` passed.

Five new real Bonsai/Window_driver tests exercise bounded billion-page opening,
coalesced asynchronous confirmation, latest callback selection, fresh editor
identity, old cancellation/completion suppression, count/current/disabled/compact/
deactivation transitions, composing errors, stale numeric revisions and shortcut selection while confirmation is pending. Commands
are controlled test completions, not actual Eio/native transport requests. A sixth
test freezes the exact public closed/open/closed transaction sequence in
`test/fixtures/pagination-view-{0,1,2}.hex` and compares it on every run.

The native TestPlatform test decodes those exact OCaml transactions and mounts
the retained host. It checks:

- Space activation of the ellipsis and a bounded native accessibility tree.
- Numeric field focus on opening and disabled Go before the first observation.
- Native arrow/Enter editing without emitting a shortcut click.
- Keyboard access to shortcuts.
- An accessibility click queued before closure cannot activate a removed shortcut.
- Closing removes the numeric owner and restores focus to the opening ellipsis.

The first native harness used keydown-only `simulate_keystrokes("space")`, which
does not trigger the button's key-release behavior. Sending actual KeyDown and
KeyUp, as the established workflow-stepper fixture does, fixes the harness. This
was not a product workaround. The first OCaml harness likewise used invalid
Changed revision zero; it now uses the actual native Observed mount event.

## Commands and current results

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/view_api/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @lib/eio/runtest examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib pagination_view_test --offline
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

These pass; the native focused run reports one passing test. The full `dune build -j2 @runtest @fmt examples/gallery/main.exe` also passes
after freezing the fixtures and fixing the Dune stanza formatting. The full native library suite passes **597 tests with two existing skips**.
Strict all-target Clippy for native/protocol with `native-image-tests` and warnings
denied also passes. Rust formatting, the catalog source audit and `git diff
--check` pass. No production Rust/protocol behavior changed in this extension.

The gallery uses the public API for actual breadcrumb path changes, passive
intermediate labels, styling, full/compact paging, count changes, disabling and
bounded page selection. Building it does not qualify its physical presentation.

No OS windows were opened. Native TestPlatform is not physical macOS keyboard,
IME, external accessibility/VoiceOver, GPU output, measured resource usage,
installed-consumer acceptance or Linux GUI acceptance. Those release gates remain
open. Chooser strings are English; pagination labels are localizable. See the
[contract](../design/pagination-chooser.md) and
[pinned source review](../catalog/navigation-review.md).
