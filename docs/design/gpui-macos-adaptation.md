# GPUI macOS resource retirement

GPUIO vendors `gpui_macos` at the same Zed revision as GPUI core:
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. These are ownership adaptations,
not a platform upgrade. Root and generated static backends patch the same crate;
Dune tracks its source tree so native archives cannot silently stay stale.

## Ownership contract

`MacWindowState` stores the AccessKit subclass adapter. That adapter owns a
retained reference to the native content view, whose GPUI subview holds an
`Arc<MacWindowState>`. Without explicit adapter retirement, this forms a cycle
after the GPUI platform window closes. The state retains the Metal renderer,
including its layer and window-sized graphics allocations.

`MacWindow::drop` now takes the adapter out of state and drops it before native
window teardown. The adapter is released outside the state mutex, since
Objective-C destruction can reenter callbacks. The platform window still owns
the native window at this point. AccessKit restores the original Objective-C
class and removes its associated adapter as part of its existing destructor.
No raw-pointer lifetime contract, input behavior or accessibility tree format
is changed. Ordinary live windows retain their adapters.

## Evidence and regression boundary

The original full lifecycle audit finished 33 closed-window checkpoints with
empty application registries and only 135 MB peak RSS, but OS physical footprint
reached 2.52 GB. IOSurface regions increased by three per window, ending at 99.
A repaired four-cycle smoke has no IOSurface category at any closed checkpoint
and physical footprint 55–77 MB. Three full repaired runs now pass all 99
closed checkpoints, with no IOSurface category and peak settled footprint below
107 MB. See [before/after evidence](../evidence/physical-memory-och17.md).

Before full repaired runs, add the explicit `--check-closed-surfaces` regression
gate to the physical audit: every closed checkpoint must have no nonzero
IOSurface category accounting, including region counts. This targets the
observed window-surface retention on the reference macOS 14.5 system. It does
not establish a complete Metal census or use a newly selected physical-footprint
budget to reinterpret the failing measurements. Preserve all category values,
RSS, physical-footprint growth and raw tool output independently. The unmodified
run must remain available as before-fix evidence.

## Native menu action ownership

The upstream platform appended every menu-bar and Dock action to one
process-lifetime vector. Replacing a menu released its native items but retained
the boxed Rust commands. GPUIO's menu snapshots avoid replacements on unrelated
renders, but actual command, scope or active-window changes still replace menus.

The platform now owns separate action maps for the installed bar and Dock menu.
Replacing either owner clears only its own map. Every command receives a
monotonically increasing signed 64-bit tag, matching `NSInteger` on the supported
macOS architectures. Tags are never reused, including across bar/Dock owners;
exhaustion disables new items instead of wrapping. The maps retain only current
actions, independent of the number of previous replacements. A callback already
in flight retains its own action clone until it returns.

AppKit clients can retain old `NSMenuItem` objects. Their old tags resolve to no
action after replacement, so they cannot invoke a new command accidentally.
Both validation and action delegates release the platform mutex before invoking
or restoring callbacks, including when the tag is missing. This avoids a stale
item query reacquiring the same mutex and hanging the UI. Native menu construction
and registry replacement remain serialized by the existing platform lock.

The portable registry regressions compile the actual vendor module on both host
platforms; the macOS `native_menus` fixture separately holds real native items
across 128 replacements and invokes their delegates. See
[menu retirement evidence](../evidence/native-menu-retirement-och41.md).
That retirement checkpoint predates the menu-bar artwork extension below.

## Native menu artwork and explicit disabled state

The subsequent menu-bar extension carries validated `gpui::MenuIcon` RGBA
snapshots on action/submenu items. `menu_icon.rs` creates AppKit-owned template
images; no encoded SVG parsing or OCaml callback runs in menu construction.
The [artwork contract](native-popup-menu.md#menu-bar-artwork) describes worker,
bitmap, snapshot and active-window ownership. The Feedback gallery demonstrates
this through the public OCaml API.

Actual menu opening exposed a separate validation bug: `setEnabled(false)` was
later overridden by AppKit's automatic validation, because a generic GPUIO
command listener existed. Explicitly disabled items now have no registered
action. They use the GPUI delegate selector even when an OS edit selector was
requested, so the responder chain cannot re-enable them. Enabled items retain
their existing selector and dispatch behavior. This also avoids retaining a
boxed action for a disabled native row; re-enabling replaces the snapshot with
a fresh, non-reused tag. The native regression calls `NSMenu.update()` before
checking disabled state, rather than checking construction alone.

## Reproduction and maintenance

```sh
python3 scripts/vendor_gpui.py --crate gpui_macos --output scratch/gpui-macos-reconstructed
```

The script verifies the upstream archive and `third_party/patches/gpui-macos.patch`
hashes in `third_party/sources.json`, materializes workspace dependencies without
changing their pins/features, preserves `Cargo.toml.upstream`, and copies the
upstream Apache license. The menu registry adds one file; all 18 reconstructed
files match the vendored tree at the menu-retirement checkpoint. The artwork
extension adds `menu_icon.rs`, bringing the reconstructed total to 19 files.
No Cargo cache files are edited. When refreshing upstream, inspect the ownership
chain and adapter destructor again, then rerun native accessibility/window-close
behavior and the physical audit. Do not remove the patch merely because registry
or RSS tests pass.

Also rerun `cargo test -p gpuio-native --test menu_action_registry` and the actual
macOS `native_menus` fixture when updating native menu construction or delegates.
Preserve the clipboard around that fixture: its existing context-menu checks
exercise Copy. Menu payload retirement is a distinct gate from window/Metal
retirement and does not replace the physical-memory audit.
