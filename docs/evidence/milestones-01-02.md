# Milestones 01 and 02 acceptance ledger

Audit date: 2026-09-24. Milestone 01 is merged. Milestone 02's remaining
implementation is in [PR #10](https://github.com/dakotamurphyucf/gpuio/pull/10),
with local acceptance complete and required hosted checks and merge pending.
This ledger maps the live Linear scope to implementation and evidence; it is not
a claim that pending checks have passed.

The owner's platform decision makes macOS functionality and Linux builds/unit
tests the development gate. Full Linux GUI, physical IME candidate-panel and
comprehensive desktop accessibility validation remain in OCH-17. Synthetic input,
native OS callbacks and physical OS gestures are distinguished in the reports.

## Foundation requirements

| Ticket | Requirements and evidence |
| --- | --- |
| OCH-18 | Public `dakotamurphyucf/gpuio`, `main`, Apache-2.0; README, CONTRIBUTING, AGENTS, engineering standards, THIRD_PARTY and versioned designs. Core abstract types, `.mli` contracts, PPX/expect examples are in `lib/core` and `test`. Root scratch is ignored and excluded by Dune. The original Zed license notice is preserved under `third_party/licenses`. |
| OCH-19 | `third_party/sources.json`, its checked patches, `gpuio.opam.locked`, `Cargo.lock` and `rust-toolchain.toml` pin stock OCaml 5.3, Bonsai/Core v0.17, Dune 3.24.2, Rust 1.97.1 and the native dependency sources. The native Bonsai subset selects libraries and renames reserved identifiers; it adds no lifecycle workaround or clock-accessor patch. Codec/lifecycle tests and package inventories are retained. |
| OCH-20 | `scripts/gpuio` and the [development guide](../development.md) provide isolated bootstrap, doctor, build, test, format, lint and editor commands. `local-validation.json` records unchanged unrelated defaults/packages; OCaml/Rust LSP navigation reports are checked in. Hosted checkouts exercise paths outside the author's machine. |
| OCH-21 | Dune builds the Rust archive from source; foundation and two-window examples exercise input callbacks, independent native state and teardown. [Foundation CI evidence](foundation-ci.json) records actual macOS, X11 and Wayland smoke coverage and its limitations. |
| OCH-22 | `.github/workflows/foundation.yml` runs both pinned environments, formatting/build/tests/lint, native compilation, required macOS functionality and informational Linux GUI checks, with environment/log artifacts. Main requires both matrix checks and an up-to-date PR; force pushes/deletion are disabled. |
| OCH-6 | Parent foundation gate: OCH-18–22 merged in PR #1. [Foundation CI evidence](foundation-ci.json) records the source/merge commits, clean-run result and two-window/input coverage. |
| OCH-7 | [Bridge contract](../design/bridge-v1.md), pure protocol crate, native tree/session/mailbox/transport and OCaml driver implement capability negotiation, generation/revision checks, atomic updates, bounded decoding/queues, acceptance versus render acknowledgement and asynchronous delivery. Paired fixtures and Rust integration tests cover malformed/oversized/stale batches, rollback, pressure, cleanup and panic containment. PR #2 is merged. |
| OCH-8 | [Typed UI contract](../design/typed-ui.md), Core view/style/theme APIs, Bonsai adapter and keyed reconciler provide layout, typed properties, reset/inheritance/precedence and GPUIX style mapping. View/appearance expect tests and `native_ui` cover identity, callback refresh, theme changes and native state/input behavior. PR #3 is merged. |
| OCH-9 | [Runtime contract](../design/runtime.md), `lib/eio/app.ml`, Bonsai driver and scoped tasks implement one owning OCaml UI domain, per-window drivers and one shared configurable Eio timer (60 Hz default), plus immediate native/task wakeups. Runtime expect tests cover lifecycle ordering, ordinary Bonsai clocks, selective cancellation and coalescing. [Measurements](runtime-och9.md) separate ticks, CPU/allocation, latency, native commits and render observations for one/four windows. Static ticks do not cause native redraws; CI checks no Async symbols. PRs #4/#5 are merged. |

Repository settings, PR merges and branch protection were re-read from GitHub on
the audit date. The current main at audit time is PR #9's merge
`8a9167cf227a290f967452c051f0b4c7dde19461`, whose run 34750544558 succeeded.
Historical artifacts establish earlier acceptance; the consolidated PR must still
pass regression checks against its final head.

## Native interaction requirements

| Ticket / requirement | Implementation and acceptance evidence |
| --- | --- |
| OCH-10 native editor | [Editor report](native-editor-och10.md) and contract describe the pinned reusable engine evaluation, stable native session/controller, grapheme editing, clipboard, selection/undo, IME marked text, auto-grow, revision-checked commands and exact submit snapshots. Native macOS input/accessibility callbacks and public two-window tests cover stale edits, submit-then-type races and disposal. PR #6 is merged. |
| OCH-11 controls and composition | `lib/core/view.mli`, control modules and [native control contract](../design/native-controls.md) cover button/toggle/checkbox/radio/Select/Combobox, Tooltip/Popover/Dialog, menus, progress and in-app notifications. `native_controls` exercises keyboard operation, semantic roles/states/actions, focus traps/restoration and popup teardown. Earlier button/radio/Select PRs #7–9 are merged; remaining families are in PR #10. |
| OCH-11 shared commands | [Menus](native-menus-och11.md), [palette](native-palette-och11.md) and [command lifetimes](command-lifetimes-och11.md) verify shared button/menu/shortcut/palette routes, scoped ownership, current state and stale rejection. Propagation/default decisions execute natively; no synchronous OCaml preventDefault is promised. |
| OCH-11 pointer and drag/drop | [Pointer](native-pointer-och11.md) and [drag/drop](drag-drop-och11.md) reports cover capture, nested routing, immutable offers, modal gating, stale callbacks and owner release. Actual AppKit gestures additionally exercise file handoff/reentry/cancel/unmount and window close/application shutdown while dragging. External filesystem transfer confirmation is not claimed. |
| OCH-11 file dialogs | [AppKit/dialog report](native-file-dialogs-och11.md) covers real file/directory/multiple/save panels, exact selected paths, capability queries, Eio reads, busy/close/shutdown behavior and weak-owner disposal. [Linux portal report](linux-file-portal-och11.md) covers bounded requests and Wayland ownership; system-libwayland tests require the Linux gate. Hosted CI compiles the AppKit AX harness and runs public lifecycle checks; actual AX selection was validated locally. |
| OCH-11 assets and styles | [Assets](assets-och11.md), [button icons](button-icons-och11.md) and [theme/scale](theme-scale-och11.md) cover bounded registration/decode/cache ownership, raster/SVG/icon views, real GPU pixels, semantic labels, retired mounted leases, corner clipping, native foreground/state styles and live theme changes. Controlled density changes cover SVG resampling without a tree revision; this is not a physical monitor-switch test. |
| OCH-11 scrolling | [Scroll report](scrolling-och11.md) covers nested transcript/ancestor routing, horizontal code versus vertical transcript, composer scrolling, popup/modal shielding, retained offsets and owner release. Tests exercise actual GPUI windows with synthetic wheel events, not physical trackpad momentum. |
| OCH-11 basic transitions / OCH-12 | [Animation contract and evidence](../design/animations.md), `motion.rs`, `animation_view.rs` and public `examples/animation` cover typed numeric dimensions/offsets/opacity/radii; initial placement, delay/duration/easing/Bezier; painted-value interruption/reversal; once-only endpoints and generation rejection. Deterministic tests and actual GPUI frames verify fixed-width sidebar content, no per-frame OCaml commits/events, repeat visibility, idle completion and unmount/close cleanup. |
| OCH-12 reduced motion | Shared System/Reduce/Full policy drives transitions and progress. Actual macOS notifications and deterministic policy tests cover live override precedence and observer disposal; private Linux portal peers cover setting changes/restarts/absence/timeouts. Reduced repeats stay static and finite transitions settle; public Bonsai/Eio integration tests the live override. |

## Consolidated delivery gate

The final local Rust workspace tests, Dune `@all @runtest @fmt`, feature-enabled
Clippy, native animation/controls/progress/scroll/image/drag/dialog checks and public
examples passed before PR submission. The individual reports state the exercised
behavior and limitations; ignored scratch logs retain detailed local output.

[Run 36031501378](https://github.com/dakotamurphyucf/gpuio/actions/runs/36031501378)
is the first consolidated hosted run, submitted for implementation head
`732ae6bc66cf3bfdcfee7894b3d0755a2b595892`:

- macOS passed formatting, source build, OCaml/Rust tests, no-Async verification,
  ordinary and native-feature lint, and every runtime/native/public GUI stage.
  Retained logs include the animation geometry/close, reduced motion, controls,
  scroll routing, GPU pixels, drag/drop and native editor/marked-text markers.
- Linux passed formatting, source build, OCaml/Rust tests, no-Async verification
  and ordinary lint. This includes three system-libwayland tests, nine deterministic
  animation tests and seventeen portal tests. Native-feature lint rejected an
  unused focus-test argument whose assertions are macOS-only. The correction
  explicitly acknowledges the unused argument on other platforms without disabling
  lint or removing assertions; local feature-enabled Clippy and formatting pass.
- Informational Wayland smoke passed foundation, two-window, bridge, typed views,
  runtime and public editor scenarios, then native radio/Select checks. Combobox
  clipboard insertion produced an empty query instead of `De`, stopping the
  remaining GUI cases. X11 was skipped after the required lint failure. OCH-17
  records this exact limitation; this run does not establish Linux GUI acceptance.

The macOS and Linux artifacts are named `foundation-logs-macOS-ARM64` and
`foundation-logs-Linux-X64`. This first run is regression evidence, **not** a passing
final merge gate: both required jobs must pass on the final PR head. The live
[PR checks and merge record](https://github.com/dakotamurphyucf/gpuio/pull/10/checks)
and OCH-11/OCH-12 completion comments provide that final delivery evidence.
