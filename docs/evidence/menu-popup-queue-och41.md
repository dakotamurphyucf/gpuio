# Native popup queued-start cancellation — OCH-41

Local work on `95e9b07`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
This adds deterministic real-AppKit scheduling evidence to the existing
[public two-window qualification](menu-multiwindow-och41.md).

`native_menu_popup_queue` creates a real GPUI/AppKit window using the production
View, session, menu commands, owner leases and run-loop scheduler. Every negative
case admits Show and invalidates it in the **same GPUI update**, before returning
the borrow. The queued `CFRunLoopPerformBlock` cannot run between these operations.
No sleep is used to guess whether cancellation beat the start of tracking.

A counter compiled only with `native-tests` increments immediately before the
AppKit popup call. Each negative case requires no increase and a retired global
lease. This distinguishes avoiding the call from a stale popup that opens briefly
and is dismissed by a later render. A positive control then requires one actual
popup call and submits normal Close after observing entry; it proves that a test
which skipped every popup would fail. It does not measure physical presentation.

The final native run passes:

- Explicit Close before queued start.
- Observer generation replacement before queued start.
- Menu definition replacement before queued start.
- Hidden and disabled owners before queued start, followed by restoration.
- Removing the full owner subtree before queued start, then remounting the same
  node slots with new generations. The new owner can open a current popup.
- Current-owner recovery: one AppKit call and ordinary asynchronous Close while
  the native loop runs.
- Window close before queued start; no later AppKit popup call.

The initial removal fixture used only `SetRoot None` with retained disconnected
nodes, which correctly failed transaction validation with `InvalidTree`. The
final fixture removes the whole subtree and creates fresh node generations.
That fixture correction did not require a production behavior change. Earlier
compiler errors were corrected before native qualification (the pinned GPUI
window title is set through `set_window_title`, not a `WindowOptions.title` field).

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native \
  --features native-tests --test native_menu_popup_queue --no-run
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native \
  --features native-tests --test native_menu_popup_queue
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native \
  --all-targets --features native-tests -- -D warnings
```

The local run is bounded by an external 90-second process-group timeout with
termination/kill and reaping on failure. The successful harness closes its owned
window and exits zero. It does not access the clipboard or VoiceOver settings.
Strict all-target Clippy with `native-tests`, Rust formatting, workflow actionlint
and the changed-document link check pass. The preceding complete OCaml/native
suite results belong to the activation fix; no OCaml code changes in this follow-up.
The CI native-build step now compiles this executable, and a separate macOS step
runs it with a deadline; hosted execution of this addition is pending. The
non-macOS executable explicitly reports a skip, not Linux native acceptance.

The [source/log archive](menu-popup-queue-och41/evidence.tar.gz) and
[manifest](menu-popup-queue-och41/manifest.json) retain the exact overlay and
final evidence. The production build has neither the counter nor harness: both
are feature-gated. No protocol or dependency version changes are required.

These tests qualify the listed ordering cases, not every possible OS event
interleaving. Public OCaml behavior, physical keyboard/clipboard, popup pixels,
and multi-window activation remain covered by their separate evidence. Native
menu-bar/nested-icon and consolidated family acceptance, VoiceOver, performance,
and final release qualification remain open.
