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
