# Focused-input query — OCH-41

Local checkpoint, 2026-10-04, macOS arm64, uncommitted worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. Dependency pins/vendor patches are
unchanged. This evidence does not establish physical keyboard/IME/AX acceptance.

## Implementation

`App.Window.focused_input` uses the existing bounded window request lane. It
returns `Window.Input.t option` or a window error, with current eligible native
input kind and exact window/node identity. It never calls an editor snapshot or
copies its value. Matching helpers compare an existing typed controller snapshot
with the observation. Read-only inputs remain eligible. Composite color text
fields map to their owning controller; color sliders/buttons are excluded.

The native query checks actual focus, live retained ownership, paint eligibility
and modal scope. Its dispatch avoids ordinary window-command observation/notify.
Public `Window.Command` retains its command-only constructors and result type,
with explicit conversion to the internal wire. Command/query wrappers reject a
response of the wrong kind. Correlation, exact window generation, 64-request
budget, closure and native failure retain the shared request handling.

The unpublished epoch-3 command adds tag9; window response adds tag2 carrying an
optional node/kind pair. Existing command and response tags stay unchanged.
See [contract](../design/window-input-query.md).

The Runtime gallery has **Which input owns focus?**. Primary+Shift+I inspects its
draft, masked and read-only inputs while they retain keyboard focus, displaying
only the returned kind. This public-API example still needs physical validation.

## Completed checks

Commands use the isolated repository environment and `GPUIO_JOBS=2`.

- `./scripts/gpuio exec cargo test -p gpuio-protocol --test window --offline --locked -j 2`:
  **7 passed**. Independent paired request/response bytes cover absent focus and
  all seven kinds. OCaml fixtures reject unknown kinds, invalid generations and
  every truncated present-input payload.
- `./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j 2`:
  **892 passed, two existing macOS private-bus skips**. Four new production-widget
  TestPlatform tests cover Input/Textarea/Combobox/OTP/Number/Palette discovery,
  metadata serialization independent of protected contents, blur, read-only,
  disabled/hidden/removed ownership, reused node slots with retained old handles,
  modal blocking and color text versus non-text controls through Tab traversal.
- `./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`:
  **passed**, including public exact-generation/kind matching and malformed input
  fixtures, and the new gallery preview. An earlier focused run needed whitespace
  correction in one new expectation; it was reviewed and edited manually.
- `./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j 2 -- -D warnings`:
  **passed**. Existing dependency future-compatibility notice for `block 0.1.6`
  and macOS duplicate-library linker warnings remain unchanged.
- `./scripts/gpuio check-fmt`, edited-document link checks and `git diff --check`:
  **passed**. The catalog audit still accounts for 146 pinned modules across 43
  families; structural coverage is distinct from complete behavioral acceptance.

Scratch logs/notepads are in `scratch/agents/root-20261003-release-notices/` and are
not build dependencies. Earlier native failures were invalid test fixtures
(single-line row limits, required combobox metadata, non-contiguous node slots,
and removal without retiring the node); final fixtures follow production admission.

## Remaining acceptance

Window-wide selected-text/selection-clear/end helpers remain separate open work,
including a bounded copy contract. Physical gallery/keyboard/accessibility,
current required Linux checks, clean-machine distribution and reviewed publication
remain milestone gates. OCH-41/OCH-17 and milestone 07 remain open.
