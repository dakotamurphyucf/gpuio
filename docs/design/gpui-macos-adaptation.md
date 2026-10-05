# GPUI macOS window retirement

GPUIO vendors `gpui_macos` at the same Zed revision as GPUI core:
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. This is a teardown adaptation,
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

## Reproduction and maintenance

```sh
python3 scripts/vendor_gpui.py --crate gpui_macos --output scratch/gpui-macos-reconstructed
```

The script verifies the upstream archive and `third_party/patches/gpui-macos.patch`
hashes in `third_party/sources.json`, materializes workspace dependencies without
changing their pins/features, preserves `Cargo.toml.upstream`, and copies the
upstream Apache license. All 17 reconstructed files match the vendored tree.
No Cargo cache files are edited. When refreshing upstream, inspect the ownership
chain and adapter destructor again, then rerun native accessibility/window-close
behavior and the physical audit. Do not remove the patch merely because registry
or RSS tests pass.
