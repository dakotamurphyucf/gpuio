# Window-wide selection helpers — OCH-41

Local macOS arm64 checkpoint, 2026-10-04, dirty worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This records TestPlatform and
build/codec evidence, not physical desktop or Linux compositor acceptance.

## Contract and implementation

The four public Eio window helpers inspect presence, read selected text, clear
selection and end dragging. They address the registered read-only text/document
selection layer, preserving native scope and logical ordering. They do not read
the clipboard or editable input values. See [contract](../design/window-selection.md).

Text limits count UTF-8 bytes including joining newlines: 65,536 by default,
0..262,144 accepted. Overflow returns `Limit_exceeded` without partial text.
Native collection snapshots callbacks/order, checks aggregate fallback allocation
before cloning and invokes renderer callbacks one at a time outside selection
entity borrows. Renderer-owned normalization is retained. A legacy callback can
still allocate its entire fragment; the output/collector bound is not a bound on
arbitrary renderer allocation. This limitation is public, not hidden by truncation.

The existing 64-request window lane supplies correlation, exact window-generation
matching, close completion and failure handling. Typed wrappers reject wrong reply
kinds; selected-text wrappers also enforce the requested limit. Pure queries skip
ordinary window geometry observation/notify; mutations request a repaint.
Response byte accounting includes the actual text payload before batching.
Unpublished epoch-3 window command tags10..13, response tags3..5 and error tag6
are additive. The ordinary public command/snapshot type remains unchanged.

The Runtime gallery includes **Selection without the clipboard**, using only
public APIs. Primary+Shift+U reads selection; Primary+Shift+E ends dragging;
Primary+Shift+Y checks presence; Primary+Shift+K clears it. Keyboard shortcuts
permit inspection without a pointer click changing the selection first.

## Validation

Commands use the isolated repository environment with `GPUIO_JOBS=2`.
Completed checks:

- Base text/selection suite: **238 passed** with the standalone manifest and the
  locked local GPUI/AccessKit/Taffy overrides documented in
  [the Base test command](document-accessibility-och17.md). Added tests cover
  exact UTF-8 and separator limits, empty/whitespace fragments, callback early
  stopping, active scopes, oversized fallback rejection, avoiding unused fallback
  copies and reentrant callbacks without holding a selection entity borrow.
- Window protocol suite: **8 passed**. Independent OCaml/Rust command/reply
  fixtures cover added tags and boundary limits. OCaml tests check malformed
  UTF-8, oversized payloads and truncated replies.
- Final full native suite: **897 passed, two existing macOS private-bus skips**.
  Four production-host tests cover local/cross-node selection, invalid/oversized
  reads, clipboard preservation, ending drag while preserving the range, source
  invalidation, modal exclusion, participant removal and password-editor isolation. Clearing
  read-only selection preserves the editor’s own selected range. Document tests
  also check exact bounds and renderer normalization for plain/source Markdown.
  The mailbox test queues
  64 maximum-sized selected-text replies, accounts their encoded bytes, drains
  within the message budget and releases every reservation.
- Full OCaml `@runtest` and the public gallery build: **passed**. The first run
  caught a reserved `effect` identifier in the new example; it was renamed.
- Strict native/protocol all-target Clippy: **passed**. Existing `block 0.1.6`
  future-compatibility and macOS duplicate-library notices remain unchanged.
- Catalog audit: **146 modules across 43 families**; this is structural accounting,
  not complete family or release acceptance. Repository formatting, edited-document
  link checks and `git diff --check` also passed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test window --offline --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --offline --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest examples/gallery/main.exe
```

## Maintained Base source

The pinned Base revision remains
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The bounded collector patch changes
`src/text_selection.rs` and its export in `src/lib.rs`. It is appended to the
maintained patch and its hash is recorded in `third_party/sources.json`:
`d3f711819f15b74c835bf7aa672f71070d843a8f1b16ced87d24d98770473d96`.

`python3 scripts/vendor_gpui_base.py --archive <verified-pinned-archive> --output
<fresh-directory>` reconstructs **235 files byte-for-byte**. The ignored standalone
Base Cargo.lock is excluded from source comparison. Normal builds use the
versioned snapshot, not scratch output. No dependency revision or switch changed.

Logs and per-ticket notes are under
`scratch/agents/root-20261003-release-notices/`. Physical selection/keyboard/AX,
installed-gallery acceptance, current required Linux nongraphical checks,
performance/resources/distribution and reviewed publication remain separate.
OCH-41, OCH-17 and the full milestone-07 goal remain open.


## Physical public gallery walkthrough — 2026-10-05

The actual macOS 14.5 arm64 gallery now passes all five native selection cases.
Harness/CI source is `8f14222`; the run used a dirty worktree based on
`999e53195418e61d2b0e3e1ac794ecda7e1821eb`, with only the new harness/workflow
changing. No library or example implementation change was needed. The existing
optimized gallery binary has SHA-256
`506e27144b26dc0172f21548e20827643deaa7c67604dae41495959e5225fb0c`;
its build and source provenance are also recorded in
[the focused-input walkthrough](window-input-query-och41.md#physical-public-gallery-query--2026-10-05).
Application source is unchanged since that build; subsequent example edits are
README-only.

1. A real pointer drag across two selectable text nodes returns exactly 89 UTF-8
   bytes, including the joining newline, `café`, `京都`, and the joined `👩‍💻`
   sequence. Native Command+Shift+U reads the public window helper and
   Command+Shift+Y reports selection presence.
2. While the pointer button is still held, Command+Shift+E ends selection.
   A further native drag event over a third line and mouse-up preserve the
   original range and exact returned text.
3. A second window reports empty selection without changing the first window's
   range. Closing the second leaves the original query functional.
4. Command+Shift+K clears selection; presence and read helpers acknowledge empty.
5. A new 39-byte selection is made before leaving the page. Returning mounts a
   fresh native selection layer and reports empty, even though Bonsai retains
   the example's prior notice model.

The test alternates presence/read acknowledgements so a stale identical status
message cannot stand in for a completed read. Empty-selection shortcuts are
invoked with focus in the card's registry; pointer selection establishes that
focus in the populated case. The first two exploratory runs incorrectly queried
an unfocused sibling registry, and a third expected Bonsai to reset its notice on
native unmount. Those harness assumptions were corrected; their failed reports
remain in local scratch. No production routing or lifecycle fix is claimed.

The clipboard's change counter remains identical across the entire run. The
harness reads no clipboard payload and performs no clipboard writes. It changes
no OS preferences, input sources or VoiceOver state. Both windows close, the
application exits zero, and its process is reaped. The final screenshot was
inspected: the first two lines are highlighted and the third is not.

```sh
python3 -m py_compile scripts/test_macos_window_selection.py
python3 scripts/test_macos_window_selection.py --output scratch/agents/root-20261004-resumed/window-selection-runtime-004
```

Python compilation, actionlint 1.7.12 and `git diff --check` pass. A scoped macOS
CI step is added; hosted execution of this new step remains pending. The ongoing
run at `999e531` does not contain it.

Retained evidence: [report](window-selection-och41/report.json),
[walkthrough log](window-selection-och41/walkthrough.log),
[application log](window-selection-och41/application.log), and
[screenshot](window-selection-och41/selected-unicode.png).
This qualifies the public read-only plain-text example; native tests above retain
separate bounds/modal/editor-exclusion coverage. It is not rendered-Markdown,
VoiceOver, arbitrary plugin, Linux GUI or whole-milestone acceptance. VoiceOver
remains on the owner's explicit hold.
