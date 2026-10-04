# Document profile workers — OCH-41

Local macOS arm64 checkpoint on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`
plus the working tree. This builds on the [profile bridge](document-profile-bridge-och41.md).
Native views still submit requests without profiles and do not install the returned
profile. No application-visible profile support or ticket completion is claimed.

## Implemented worker behavior

`document_profile_jobs::Request` owns a checked binding and immutable configuration.
The document pool compares the complete configuration when deciding whether work
is unchanged. Code/Diff requests with profiles fail before altering existing work.
Unpublished sources wait without configuring a profile. Updated configurations
cancel superseded requests; late results cannot replace current work.

Before configuration, the existing 64 MiB pool reserves ordinary source/AST work
plus declared opaque retention, generated strings, displayed-text projection,
highlight vector capacity and adapter metadata. Prepared results reduce that charge to a checked retained allowance: declared
opaque state, generated public strings, displayed-text size, plugin metadata, code
keys and actual highlight vector capacities. The displayed-text counter is reused
without materializing a plain-text copy. Excess backing capacity rejects the whole
result; cancellation, discard and teardown release all charges. These conservative
admission units are distinct from allocator RSS. Trusted hooks can allocate before
returning, so this is not an allocation sandbox or a physical memory qualification.

Configuration, SDK Markdown/HTML parsing and custom highlighting run in the worker.
SDK preparation returns a Parse/Highlight stage with its typed error; Configure
is reported separately by the host. Failures preserve source fallback without
retaining a live failed profile. Timing counters separate configuration, parsing
and highlighting rather than counting custom highlighting as parser time.

Custom and built-in highlight runs have separate representations. Native conversion
preserves arbitrary validated font weight, strikethrough, underline, italic and
both colors. Empty custom output means plain code. Repeated identical blocks share
the custom result; a later duplicate cannot overwrite it with built-in coloring.
The worker serial supplies a positive parser epoch for later UI installation.

## Validation

Commands used the repository's isolated toolchain, without changing switches or
opening OS windows:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-document-sdk
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-document-sdk \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
```

Full native **862 passed**, with two existing macOS private-bus skips. SDK **15
passed**. Strict lint passes after correcting an unused alias in a new test fixture.
The nine new worker tests cover:

- A real separate worker thread, complete Unicode styling and duplicate blocks.
- Unchanged-request silence, epoch replacement and obsolete completion rejection.
- Configure/highlight panics and malformed Unicode ranges with precise failure stages.
- Unpublished sources and capacity rejection without invoking factories.
- HTML parsing and safe image replacement with profile highlighting.
- Invalid-mode atomicity and malformed-MDX parser failures.
- Cancellation while configuration is running, with bounded channel synchronization.
- All 128 small profiles retained within the shared budget, with complete charge/plugin release.
- Excess backing vector capacity behind a one-run result rejecting the whole picture.

The initial Unicode fixture incorrectly included the code fence's trailing newline;
the pinned reader strips that newline. The corrected assertion checks the six UTF-8
bytes of `世界`. All final native cases pass. The gallery links and formatting checks pass. Full native862 and strict lint were
rerun after the final displayed-text accounting adjustment; both pass. Additional
commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

The catalog audit is structural evidence only. GPUI Base remains identical to the
previously verified 234-file reconstruction. No new physical-platform evidence is
claimed.

## Remaining attachment work

Connect profile bindings to Presentation requests, then install profile, prepared
text, extensions and highlights together. Retained render closures must retain the
same resource charge until they are dropped. Implement source/configuration/input/
visibility/modal/unmount event-lease guards, once-only failure delivery and stable
image-closure refreshes. Qualify native code/table/inline/block rendering, focus,
selection/search/copy and accessibility, then add the public gallery and installed
profile-package consumer. Application defaults and the remaining OCH-41/OCH-17
catalog, physical platform and release requirements remain open.
