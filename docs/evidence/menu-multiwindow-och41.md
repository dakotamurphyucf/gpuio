# Native popup window and editor ownership — OCH-41

Local source overlay on `23650c82c647ed4b54b134ffe086dc6ee5743d24`,
2026-10-06, macOS 14.5 arm64 / M1 Max. This is scoped native lifecycle
qualification; OCH-41 and milestone 07 remain open.

## Failure and correction

The new public `menu_controller --two-windows` example gives each window its own
Bonsai graph, menu controller, command registry and two native text editors.
Its [walkthrough](../../examples/menu_controller/multiwindow.md) explains these
boundaries without requiring Rust knowledge.

The initial native sequence passed inactive-window Show rejection, independent
Run counters, editor focus changes/removal and closing the popup's owner while
the other window remained usable. Adding activation transfer exposed a failure:
after activating B during A's tracking popup, A's menu remained open past the
10-second observation deadline. A stronger reproduction issued public Activate
and Observe commands: B reported `active=true` and A reported `active=false`,
yet A's actual AppKit popup remained visible. This distinguishes the failure
from AXRaise merely changing window order without activation.

The window activation observer previously cancelled chart input but did not
retire native popups. It now calls `close_platform_popups` for the inactive
owner. This captures tracking menu IDs, closes each without restoring focus,
and publishes the closed observation. Dropping the existing native lease
invalidates selection immediately and schedules AppKit cancellation outside
the View borrow. No protocol, public API or dependency version changed.

## Actual native qualification

`scripts/test_menu_multiwindow_macos.py` passes against both the repository
binary and a fresh consumer linked against staged installed public libraries:

- An inactive B cannot open a popup while A tracks: Show returns Unavailable.
  A's native Run still dispatches only to A. Activating B then permits its own
  popup and counter update without changing A's counter.
- Both editors have nonempty native selections. Copy first succeeds from A's
  first editor. Opening again, moving focus to its second editor through the
  public controller, and choosing the still-enabled native Copy row leaves
  a test clipboard sentinel unchanged. Reopening then copies the second editor.
  Thus rejection is not explained by an empty or unavailable replacement target.
- Removing the first editor while its popup tracks, then focusing the retained
  second editor, likewise cannot retarget the old Copy. Reopening recovers and
  copies the second editor's selection.
- Activating B during A's popup confirms both native activation states, dismisses
  A's popup, and permits a new native Run in B. A's counter remains unchanged.
- Closing A while its popup tracks leaves B alive and able to open and invoke
  another popup. Closing B exits the process with status zero.

The driver inspects actual AppKit menu rows and selected children and uses native
keyboard typeahead/Enter. Setup and deliberate mid-tracking state changes use AX
buttons invoking public OCaml APIs. The clipboard's original readable payloads
are preserved in memory and restored; all test applications are reaped. This
does not constitute a VoiceOver or physical pointer-activation walkthrough.

Binary SHA-256:

- Repository: `a460e663f05d268f168d2f4e85a134a4bfe6265902bed1761ada5d7ec200e5d9`.
- Installed consumer: `266b38126b73d3ee2587753f9e39fd09e48bd504b1e5fca98d21bb4ad90354d8`.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/menu_controller/main.exe
python3 scripts/test_menu_multiwindow_macos.py
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example menu_controller \
  --workspace scratch/agents/root-20261004-resumed/popup-multiwindow-consumer
python3 scripts/test_menu_multiwindow_macos.py --binary \
  scratch/agents/root-20261004-resumed/popup-multiwindow-consumer/consumer/_build/default/main.exe
```

The independent build stages a local prefix; it does not install into or modify
an opam switch. A dedicated macOS CI step now runs the regression. Its hosted
execution remains pending; the already-running earlier-source CI run cannot
qualify these changes.

## Checks and limits

The complete native library suite with `native-image-tests` passes **983 tests,
two existing ignored**. Full OCaml `@fmt @runtest` and installed-library build
checks pass. This feature set does not include the extra canvas-only tests.
Strict all-target native Clippy with the same feature set, Rust formatting,
Python compilation and workflow actionlint pass. The independently installed
consumer also passes the existing single-window positioned-menu walkthrough.
The [archive](menu-multiwindow-och41/evidence.tar.gz) and
[manifest](menu-multiwindow-och41/manifest.json) retain source overlays, the
original failures, commands and final verification logs.

Queued-start cancellation before AppKit tracking begins, broader native menu-bar
and nested-icon qualification, consolidated catalog acceptance, VoiceOver and
performance/release gates remain separate work. This local evidence does not
claim Linux execution or desktop qualification.
