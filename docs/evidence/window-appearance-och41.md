# Native appearance observation — OCH-41

Local checkpoint, 2026-10-04, macOS arm64, uncommitted worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. These checks use GPUI TestPlatform and
the isolated repository toolchain; no OS window or system preference was changed.
They do not establish physical macOS or Linux appearance reporting.

## Contract and implementation

`Window.Appearance` exposes Light, Vibrant_light, Dark and Vibrant_dark, plus
`is_dark`. Existing window snapshots, `on_change` and Observe responses now carry
the resolved native appearance. Application color tokens remain independently
owned; `set_theme` does not force native OS decorations. See the
[contract](../design/window-appearance.md).

The host retains one GPUI appearance subscription per window owner. Its callback
captures a weak owner and uses existing asynchronous, coalesced WindowChanged
delivery. Initial watch includes actual current appearance, even before first
paint. The change adds neither a timer nor a synchronous OCaml callback.

The paired unpublished epoch-3 snapshot layout appends tags 0..3 in public variant
order. Old snapshots without the field are invalid; rebuild both runtimes together.
Window identity/generation and pending-request handling are unchanged.

The public gallery adds **Follow system** beside the explicit Light/Dark toggle.
Each window keeps its own preference; startup remains explicit Dark. The existing
observation handler now supplies appearance as well as custom-chrome metadata.
Bonsai updates the application palette when its effective light/dark choice changes.
Neither a native observation nor a color-token update recreates the editor owner.

## Completed local checks

All commands below use `GPUIO_JOBS=2` and `./scripts/gpuio exec` unless stated
otherwise. Logs reside under ignored
`scratch/agents/root-20261003-release-notices/`; they are not build dependencies.

- `cargo test -p gpuio-protocol --test window --offline --locked -j2`:
  **9 passed**, log `window-appearance-protocol-002.log`. Independent Rust byte
  fixtures cover all four appearance tags and updated command/lifecycle snapshots.
- `dune runtest -j2 test/protocol test/gallery test/view_api`: **passed**, log
  `window-appearance-ocaml-001.log`. Independent OCaml fixtures decode all variants,
  reject every truncated snapshot and unknown appearance tag4, and check `is_dark`.
  Preference expectations cover absent initial metadata, ordinary/vibrant variants
  and explicit overrides. The reconciler test requires a real Set_style update
  while prohibiting editor creation/removal or Set_text on repeated palette changes.
- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2`:
  **900 passed, two existing macOS private-bus skips**, log
  `window-appearance-full-native-001.log`. Three new production-observer tests
  cover initial non-default appearance before first paint, all variants, latest
  metadata coalescing, unchanged paints, separate native windows, subscription
  replacement/drop, and a queued deferred callback followed by immediate close.
  Weak-owner release and continued notification of the other window are checked.
  Native editor draft, selection and focus survive multiple appearance changes;
  subsequent typing still replaces the selected draft. The two-window fixture
  uses independent transports; it is not a new shared-bridge routing proof.
- `dune build -j2 @runtest examples/gallery/main.exe`: **passed**, log
  `window-appearance-full-ocaml-001.log`. This includes the independent gallery
  backend and full OCaml test aliases; no desktop walkthrough is implied.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`:
  **passed**, log `window-appearance-clippy-001.log`. The existing dependency
  future-compatibility notice for `block 0.1.6` and duplicate-library linker
  warnings remain unchanged.
- `./scripts/gpuio check-fmt`: **passed**, log
  `window-appearance-format-check-001.log`. Catalog structural coverage remains
  146 modules across 43 families; this is not full behavioral acceptance.

## Maintained source change

Only GPUI test support changes: `VisualTestContext.simulate_appearance_change`
exposes its existing TestWindow notification and normal deferred callback. Native
appearance observation uses the already-pinned production implementation.

Zed remains `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. The updated maintained
GPUI patch SHA-256 is
`2bb8cc1ae6303e86bc5d176f6799622e7df53d5bef0addf1572fa5c31506d7cd`.
Base remains `84f57fdfcb4910623fb0bb7f795b077e249f9271`, with unchanged patch
`d3f711819f15b74c835bf7aa672f71070d843a8f1b16ced87d24d98770473d96`.

`python3 scripts/vendor_gpui.py --archive scratch/agents/root-20260928-m7/zed-a57ba9b.tar.gz --output scratch/agents/root-20261003-release-notices/appearance-gpui-reconstructed-001`
passed. The script verified archive/patch hashes; a recursive SHA-256 comparison
matched **156 files**, excluding local ignored Cargo.lock/target artifacts.
Logs: `window-appearance-reconstruction-001.log` and
`window-appearance-reconstruction-comparison-001.log`. Pins and lockfile package
versions did not change for this addition.

## Remaining acceptance

The physical macOS check below now covers Follow system and explicit palettes
across two windows while editing. Linux backend observations
remain part of deferred desktop qualification, with nongraphical builds still
required. This is not high-contrast preference support or a global OS override.

The nested style/theme audit, full gallery/input/accessibility/performance and
distribution/review gates remain open. OCH-41/OCH-17 and milestone 07 remain active.

## Physical macOS appearance — 2026-10-05

The real macOS 14.5 arm64 walkthrough passed all eight cases. Its tested worktree,
based on `3d1f64b`, was committed as
`24445a08ee54dacc05420b9e039ad25a8d6c479f`; the unchanged report truthfully records
the pre-commit revision and `dirty=true`. The optimized gallery executable has
SHA-256 `1cb18f072468b16674aee7aaa3029a7ac0904554cd7067c5279e8302ea9f15af`.
No native library or renderer fix was needed.

The gallery's opt-in `--trace-window-appearance` records its existing public
window observations through Eio. The runner waits for fresh observations from
both windows after each OS switch, then independently checks actual window pixels
and existing editor handles. It does not accept a changed preference/button label
as proof of painting, or an assertion made before the native observation arrives.

- Follow system initially resolves Dark, repaints Light when macOS changes, and
  repaints Dark on the return change. Both windows publish the native changes.
- The focused first-window editor retains `Retained λ🙂 appearance draft` and the
  `λ🙂` selection through both changes. Without requesting focus again, native
  Backspace, undo, typing `a`, and undo all reach that editor and restore the draft.
- The second window's independent draft and explicit Light palette survive an OS
  return to Dark. After it closes, an explicit Dark choice in the first window
  survives OS Light. Selecting Follow system again adopts current OS Light.
- The last window closes and the owned process returns zero and is reaped.

[Light](system-appearance-och41/system-light.png),
[Dark](system-appearance-och41/system-dark.png), and
[explicit Dark under OS Light](system-appearance-och41/explicit-dark.png) captures
were visually inspected. They show the retained Unicode selection and the expected
content palettes; the last also shows light native chrome around dark application
content, as the contract specifies. All seven captures, the
[raw report](system-appearance-och41/report.json),
[native observation log](system-appearance-och41/application.log), and
[restoration record](system-appearance-och41/appearance-recovery.json) are retained.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/gallery/main.exe @fmt
python3 scripts/test_mac_system_appearance.py
python3 scripts/test_macos_system_appearance.py --output scratch/agents/root-20261004-resumed/system-appearance-runtime-001
```

The runner saves and fsyncs a recovery record before changing OS appearance, then
restores and verifies the original effective Dark mode and automatic-switch
preference, including absence. This run began and ended in Dark with the automatic
preference absent. Success is recorded only after restoration. Four portable tests
cover normal/error/interruption paths, absent/true/false automatic preferences,
restoration ordering, failed readback, and refusal to overwrite a recovery record.
An interrupted process that cannot execute cleanup can be recovered explicitly:

```sh
python3 scripts/mac_system_appearance.py --restore PATH/appearance-recovery.json
```

Build/format, portable tests, Python compilation and workflow actionlint pass.
Both portable restoration checks and the physical walkthrough are wired into CI;
their new hosted execution remains pending. This does not qualify automatic
scheduling, high contrast, vibrant variants, Linux desktop appearance, or VoiceOver.
VoiceOver, input-source and clipboard settings were neither changed nor exercised.
