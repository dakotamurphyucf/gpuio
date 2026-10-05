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
the static field label and returned kind by matching against existing controller
snapshots, without reading an editor value. Physical validation is recorded below.

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

## Physical public-gallery query — 2026-10-05

All five real macOS 14.5 arm64 cases passed. The tested worktree based on
`8722409` was committed as `138b56a`; the unchanged
[report](focused-input-och41/report.json) records its pre-commit revision and
`dirty=true`. The optimized gallery executable SHA-256 is
`506e27144b26dc0172f21548e20827643deaa7c67604dae41495959e5225fb0c`.

The gallery uses `Window.Input.same_text_input` against each already-observed
controller snapshot to name Draft, Masked value or Read-only value. The operation
compares window/node generations and kind; no text accessor or new editor-value
request is used. This also lets the test distinguish actual responses for the
three controls instead of accepting a stale generic `Input` notice.

The foreground walkthrough issues real Command+Shift+I shortcuts and checks:

- The ordinary editor returns Draft while preserving its Unicode text, `λ🙂`
  selection and focus. Subsequent native Backspace/undo still edits that owner.
- The masked editor returns Masked value while retaining focus; the test does
  not request its AX value. The query displays only the fixed label and kind.
- The read-only editor is eligible, returns Read-only value and remains unchanged
  after native Backspace.
- A second window matches its own controller and independent draft; closing it
  leaves the first window's query functional.
- Leaving and returning to Runtime mounts a fresh native editor with its initial
  draft, and the next query matches the current controller rather than a retired
  identity. Both windows close; the process returns zero and is reaped.

The [inspector capture](focused-input-och41/focus-inspector.png) was visually
inspected: all three controls are visible, the masked value remains masked, and
the read-only owner notice is correctly rendered. The
[application log](focused-input-och41/application.log) is also retained.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/gallery/main.exe @fmt
python3 -m py_compile scripts/test_macos_focused_input.py
python3 scripts/test_macos_focused_input.py --output scratch/agents/root-20261004-resumed/focused-input-runtime-001
```

Build/format, Python compilation and workflow actionlint passed. The walkthrough
is wired into macOS CI; its new hosted execution remains pending. No library API,
native renderer, OS preference, input-source, clipboard or VoiceOver changed.
This physically qualifies these three single-line owner cases; the other input
kinds, exclusion rules and malformed protocol cases retain their separately
scoped coverage above. It is not whole-gallery or VoiceOver acceptance.
