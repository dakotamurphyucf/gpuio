# Public horizontal managed-list integration — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`. Builds use the
repository's stock OCaml 5.3/Bonsai v0.17 toolchain. No shared switches changed.
This follows the [engine foundation](horizontal-list-engine-och41.md) and
[accepted contract](../design/horizontal-managed-lists.md). No OS windows opened;
TestPlatform semantics/input do not constitute physical macOS/VoiceOver/GPU or
Linux desktop qualification.

## Implemented behavior

- Checked Core `Axis`/`Extent`, `Config.horizontal ~width`, axis/extent accessors;
  existing `Height` and vertical constructors remain source-compatible. Fixed
  item wrappers clip along the configured axis. Estimates become native measures.
- Appended Op109 pairs independent OCaml/Rust encodings; old records/tags are
  unchanged. Strict decoding rejects invalid enum/truncated/trailing payloads.
  Tree admission rejects non-list targets, horizontal native tree input and
  horizontal managed tables atomically. Axis updates retain logical metadata.
- Production Host uses physical width for horizontal placeholders/row sizing,
  visibility, reveals, cross-resize invalidation, scrollbar offsets and wheel
  ownership. Root accessibility records orientation. Commands remain serialled,
  asynchronous and axis-independent. Data/layout callbacks never call OCaml.
- Axis/config changes replace the native measurement owner while retaining a
  surviving anchor and tail pause state. Old custom scrollbar capture/drag hooks
  and range focus retire first. The replacement uses the current native handle.
- Reactive `component_with_config` and `paged_with_config` keep surviving row models
  and controller generation. Static constructors delegate to the same implementation.
  Config changes invalidate the current geometry observation, preventing stale
  boundary-driven paging; retained requested/pinned rows stay within budget.
- Collections → Horizontal cards uses only public APIs, with 10k unequal-width
  cards, local reactions, first/middle/tail navigation, streamed growth, reorder,
  prepend/append and axis toggle. Structural edits run only in requested actions,
  not on every viewport update. Shared scrollbar controls apply to the preview.

## Behavior checks

Core expect tests validate finite extent bounds, legacy aliases, row identity and
style changes without node/order recreation. Bonsai tests preserve a local row
counter and previously obtained controller across three axis changes. The Eio
paging regression begins horizontal, fills cursor-advancing empty pages, changes
axis and waits for fresh geometry before another fetch; failure/retry and obsolete
generation fencing still pass.

Production Host TestPlatform checks cover 100k logical items with at most 12
requested rows, actual unequal widths/physical bounds, streamed anchor growth,
cross resize, prepend, perpendicular wheel no-op and main-axis scrolling,
following/end and duplicate-command fencing. Both-axis custom scrollbar tests
cover range focus, live thumb capture, replacement and axis changes; tree input
keeps its independent focus. These are bounded-materialization checks, not an RSS
benchmark or physical input acceptance.

## Validation

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless shown otherwise.

| Command | Result |
| --- | --- |
| `cargo test --offline --locked -j2 -p gpuio-protocol` | 359 passed |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib` | 796 passed, two existing private-D-Bus skips |
| `cargo test --offline --locked -j2 -p gpuio-native --test lists --test tables --test tree_input` | 19 passed, including valid tree/table atomic rejection |
| `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass |
| `dune build @runtest examples/gallery/main.exe` | Pass on final OCaml/gallery sources |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` (direct) | Pass |
| `python3 scripts/audit_component_catalog.py` (direct) | Structural audit pass; not functional/platform certification |
| `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-horizontal-gallery-20261003` (direct) | Pass, `run=False`; staged installed public OCaml libraries and fresh external consumer |

`git diff --check` passes. The pinned GPUI source remains exactly equal to the
foundation's reconstructed output (`diff -qr`); this integration adds no vendor
patch changes. Its verified patch hash remains
`225416601f67074d3c2abbed689770095773afd2074780e584e0a48da0062db9`.
Existing `block` future-compatibility and duplicate system-library link advisories
remain. No tests or snapshots were automatically promoted.

OCH-41 and OCH-17 remain open. Required Linux build/unit/private-bus/consumer checks
and all physical macOS release gates are separate; deferred Linux desktop
qualification remains OCH-47. See `docs/status.md` for the milestone-wide remaining
work. These local results do not claim CI, review, merge or publication.
