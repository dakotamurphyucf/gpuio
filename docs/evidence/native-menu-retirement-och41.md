# Native menu action retirement — OCH-41 / OCH-17

Local macOS arm64, 2026-10-07, based on `78a001f9`. While tracing ownership for
native menu-bar artwork, an actual AppKit regression found that replacing the
menu bar retained the previous boxed Rust action. The second replacement left
the old payload's strong count at one instead of zero.

## Cause and repair

The pinned macOS platform appended menu-bar and Dock actions to one vector and
used its indices as native item tags. Replacing or clearing menus never removed
old actions. GPUIO's snapshot comparison avoids unrelated streaming replacements,
but actual command/scope/window changes still grew that registry.

The [maintained platform adaptation](../design/gpui-macos-adaptation.md) now has
separate maps for current bar and Dock actions. Replacing one clears only that
owner's commands. Tags increase across both owners and are never reused; tag
exhaustion disables new commands instead of wrapping. Registry storage depends
on current menu sizes, not replacement history. An executing callback keeps its
own action clone until it returns.

Retained old `NSMenuItem` objects cannot resolve to replacement commands. Both
native delegate paths release their mutex before restoring the callback, even
when a stale tag has no action. The previous missing-tag branch would otherwise
attempt to acquire the same mutex twice. Command routing, protocol and OCaml
APIs are unchanged.

## Validation

The extended `native_menus` fixture passes on actual AppKit:

- 128 menu-bar replacements release every previous payload, even while the
  fixture retains the previous native item.
- Both real native delegates reject stale items without dispatch or hanging.
  Every current item dispatches its expected revision.
- A Dock action stays callable across bar replacements; clearing each owner
  releases its last payload and rejects its retained native item.
- Existing nested navigation, disabled/separator behavior, context-menu Copy,
  1,000-command virtualization, placement, popover routing, active-window/scope
  routing and disposal checks also pass. The owned test process exits zero.

The retention checks call real AppKit delegates programmatically. They do not
claim physical menu clicking, menu artwork, screen-reader operation or a process
RSS/physical-memory bound. The existing fixture performs Copy and was run
directly; the pre-test clipboard was not captured or restored in this run. Use
the repository's clipboard preservation wrapper for subsequent invocations.

Three portable regressions compile the exact vendor registry through
`rust/native/tests/menu_action_registry.rs`: 1,000 replacements with drop
witnesses, independent Dock/nested-action retirement, and exhaustion with no
wrapping or payload retention. All pass locally. These tests are included in
normal Cargo test discovery on both hosts; no new-source Linux execution is
claimed yet.

The native library suite passes **1,184 tests with two existing skips**. Strict
workspace/all-targets Clippy also passes, including native-test, canvas/image and
presentation-diagnostic features. Existing upstream dependency warnings remain;
this is not a warning-free third-party build. Formatting passes after correcting
one assertion's wrapping. All 18 reconstructed macOS vendor files match exactly.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --test menu_action_registry
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-tests --test native_menus
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --lib --features native-canvas-tests,native-image-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j2 \
  --workspace --all-targets \
  --features gpuio-native/native-tests,gpuio-native/native-canvas-tests,gpuio-native/native-image-tests,gpuio-native/presentation-diagnostics \
  -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
python3 scripts/vendor_gpui.py --crate gpui_macos \
  --archive scratch/zed-a57ba9b.tar.gz --output scratch/menu-macos-reconstructed
```

The baseline's earlier fixture version checked payload retirement and failed on
replacement two. The fixed fixture additionally checks actual stale/current
delegates. The first repair build failed because Cocoa's `NSInteger` alias is
`i64`, while the initial registry used `isize`; the standalone test wrapper also
had an incorrect relative path. Both were corrected before the passing runs.
No failed compilation or stale executable is counted as repaired behavior.

[Logs and source](native-menu-retirement-och41/reports.tar.gz) are indexed by a
[SHA-256 manifest](native-menu-retirement-och41/manifest.json). Native menu-bar
artwork, consolidated catalog coverage and full resource/release acceptance
remain open. OCH-41 and OCH-17 remain In Progress.
