# Scrollbar preference snapshot — OCH-41

Checkpoint 2026-10-04, macOS arm64. Working tree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`; substantial uncommitted changes mean
that HEAD alone does not identify these sources. OCH-41/OCH-17 remain open.

## Source and contract

The [contract](../design/scrollbar-preference.md) adds a single asynchronous
desktop query and an explicit Collections gallery action. No dependencies,
vendor patches or global toolchain settings changed in this slice.

Pinned Zed `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` implements
`App::should_auto_hide_scrollbars` by forwarding to the platform. The macOS
implementation in `crates/gpui_macos/src/platform.rs` reads
`NSScroller.preferredScrollerStyle`. Linux's `crates/gpui_linux/src/linux/platform.rs`
initializes `auto_hide_scrollbars` to false and only exposes its getter; no
updater was found in the pinned Linux source. We return `Unsupported` on Linux
instead of treating this default as a measurement. Component's snapshotted
[`sync_scrollbar_appearance`](../catalog/sources/component-theme.rs.txt)
is likewise an explicit read/sync operation, not a subscription.

The query reports resolved overlay/legacy behavior, not macOS's raw three-way
System Settings choice. The gallery maps it to Scrolling/Always respectively,
preserving manual choice on error. It uses a fresh child scope on activation,
checks that scope before applying a response, and guards one pending request.
Explicit mode actions also check scope/busy state when executed. Departed-page
responses cannot update a new activation. No automatic refresh or polling is
claimed; the example asks the user to apply the preference again after changes.

## Validation

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless stated otherwise.
Scratch logs are under `scratch/agents/root-20261003-release-notices/` and are
not build dependencies.

- `dune build -j2 @runtest examples/gallery/main.exe`: passed, including the
  full OCaml suite and independent gallery backend. Log
  `scrollbar-preference-ocaml-001.log`. OCaml checks independent request/event
  bytes, both returned styles and invalid tags, truncation, extra bytes and
  zero correlation.
- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2`:
  **906 passed, two existing macOS private-bus skips**. Log
  `scrollbar-preference-native-001.log`. The new policy test reads each snapshot
  afresh and never invokes an unsupported getter. The production dispatch test
  services two reserved requests through TestPlatform, preserves correlation,
  uses no desktop identity or window, and leaves no output after draining.
- `cargo test -p gpuio-protocol --offline --locked -j2`: **390 passed**, including
  eight desktop tests. Log `scrollbar-preference-protocol-001.log`. Rust fixes
  the same independent bytes, preserves all preceding request fixtures and
  checks malformed/truncated/trailing request input.
- `./scripts/gpuio check-fmt`: passed, log
  `scrollbar-preference-format-check-001.log`. Only formatter output was
  promoted; no expectation output was promoted. Catalog audit, relative links
  in seven edited documents and `git diff --check` passed. Structural catalog
  coverage remains 146 module entries in 43 families.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`:
  passed, log `scrollbar-preference-clippy-001.log`.

The existing `block 0.1.6` future-compatibility notice and macOS duplicate-library
link warnings remain. The tests ran locally on macOS; no fresh Linux execution,
OS preference read, physical window or hosted check is included.

## Remaining acceptance

TestPlatform's getter is a fixed false. Dispatch tests therefore establish
routing and ownership, not a read of the developer's actual macOS setting.
Physical System Settings changes, both style results, applying while scrolled,
keyboard/AX presentation, page departure and multi-window walkthrough remain
unverified. Local source guards are not physical lifecycle evidence.
Linux nongraphical checks still need execution on Linux; Linux desktop
qualification remains OCH-47. The full gallery and release gates remain open.
