# Cached native text selection — OCH-17

Follow-up to `d66f492b`, local macOS arm64. The previous checkpoint's real cached
Base TextView diagnostic lost selection and performed extra renders. This change
addresses that native selection-controller lifecycle; it does not certify the
complete accessibility or release checklist.

## Implementation

GPUI records native lifecycle callbacks as part of each paint range. Each callback
runs on fresh paint and on actual cached replay, preserving relative order and
moving between frames instead of accumulating. Retired frame callbacks release
their captures. Base selection participants register through this mechanism with
weak native handles and shared geometry. There are no synchronous OCaml callbacks
or extra element layout passes.

Automatic participant order now belongs to the window's selection generation,
instead of an application-global counter reset only by fresh layer prepaint.
Each cached or freshly painted participant gets its current order. The cached
logical-order method keeps explicit orders explicit. The low-level immediate
registration method retains its prepaint-compatible, caller-managed lifecycle;
review of Base's fixtures caught that compatibility requirement. Scope markers replay balanced native scope entry
and exit; geometry is copied only if its effective scope changes. The layer also
replays finish-frame scheduling, while GPUI continues to replay input listeners
normally. The generation sweep still clears participants missing from the frame.

The native hook's contract permits lifecycle metadata only, not drawing,
registering more paint callbacks/listeners, or depending on transient GPUI
view/style/element stacks. Content/geometry changes still require ordinary entity
invalidation. See the [design contract](../design/rendered-document-selection.md).

## Behavioral evidence

`rust/native/src/document_cached_text_test.rs` tests real `TextViewState` and
`TextSelectionLayer`, not synthetic painted selection claims:

- Eight combinations: accessibility active/inactive, selection layer inside or
  outside the cached entity, and direct/deferred text. The text has a nondefault
  selection scope. Fresh paint selects `Cached 世界`; four parent-only redraws
  preserve the exact native selected text and leave the render counter unchanged.
- With accessibility active, a native SetTextSelection request after reuse selects
  `Cached`; subsequent reuse retains that new range without another render.
- Removing the participant clears selection. Remounting the retained text entity
  does not resurrect it, including when its selection layer was removed too.
- A cached TextView and uncached generic plain-text participant copy in current
  paint order. Moving the absolute plain-text participant before the cached text
  changes Copy order without changing the cached text's geometry or render count.

`rust/native/src/document_cached_registration_test.rs` additionally pairs a raw
prepaint registration with a cached explicit-order participant. Copy follows their
logical order rather than paint order, cache renders stay unchanged, and removed
cached content does not regain selection when remounted. This preserves existing
low-level callers while offering cached logical-order registration.

These are TestPlatform native behavior tests. Their selected-text assertions use
the same native controller as Copy, but are not physical clipboard or VoiceOver
acceptance. The original failing reproduction remains in the
[previous evidence bundle](cached-accessibility-owners-och17.md).

## Validation

The full native command, using `GPUIO_JOBS=2 ./scripts/gpuio exec`:

```sh
cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests
```

**1,173 passed, 2 ignored.** The completed source also passed:

- Strict workspace/all-targets Clippy, including native canvas/image tests and
  presentation diagnostics, with `-- -D warnings` (38.85 seconds).
- `cargo fmt --all -- --check` and explicit edition-2024 rustfmt checks for the
  changed GPUI/Base vendor files.
- `cargo test --offline --locked -j2 -p gpuio-native --features native-tests --test native_editor --test native_document`:
  actual macOS editor/document checks (39.61 seconds), with clipboard preservation.
- `cargo test --offline --workspace --locked -j2` (138.82 seconds).
- `dune build -j2 examples/gallery/main.exe` (41.96 seconds).
- `python3 scripts/test_macos_text_selection.py --rendered-only --output <fresh-scratch-directory>`
  (5.89 seconds): actual macOS heading `[33,17]`, CJK `[91,2]`, joined emoji
  `[96,11]` and code `[392,7]` UTF-16 ranges, exact native Copy and stable caret
  `[392,0]`. Owned application exit 0; clipboard restored and verified. VoiceOver
  settings were untouched. Gallery binary SHA-256:
  `f702253e2feb31915afc055681bbd7e979dce692e05ac41288517a857ae0ff90`.

The real OS run is a regression check against the public gallery; cache reuse
itself is proved by the separate native render-counter fixtures, not inferred
from the OS test. These durations are validation timings, not performance budgets.
The bundle records full commands and logs. Full Dune/hosted/Linux release gates
are not replaced by these checks.

Exact vendoring reconstruction matches all 159 GPUI and 243 Base files, excluding
generated `Cargo.lock`. GPUI patch SHA-256:
`71523f2cf1b776514d35fa84f7d6da6a5e98948cbc83ba50b59e66b340cd2b69`.
Base patch SHA-256:
`b4267c3ff9850155b54d5138984862284adf83d9be4d3389014634f1cbbba82d`.

Dedicated rollback/context/dirty-source qualification, VoiceOver, full catalog,
performance/resource, distribution/provenance and required current-source Linux
checks remain open. Direct upstream GPUI/Base unit suites have not been qualified
by this checkpoint; these dependencies are exercised through GPUIO native tests.

Evidence: [reports.tar.gz](cached-text-selection-och17/reports.tar.gz) and
[SHA-256 manifest](cached-text-selection-och17/manifest.json). The bundle preserves
candidate failures/results, final logs, implementation/source snapshots and the
real macOS report. The tests ran on a dirty tree based on `d66f492b`; the archived
patch and source snapshots identify the tested implementation before commit.
