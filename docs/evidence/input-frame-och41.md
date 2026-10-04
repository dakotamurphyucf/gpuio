# OCH-41 input frame checkpoint

Local dirty worktree, macOS arm64, 2026-10-01. No commit, hosted CI, real OS window
or release acceptance is claimed. See the [contract](../design/input-frame.md).
Use the isolated repository environment and `GPUIO_JOBS=2`.

Passing focused checks:

- `./scripts/gpuio exec dune build -j2 lib/core lib/bonsai` and
  `./scripts/gpuio exec dune runtest -j2 test/view_api`: public constructors and
  three frame expectations, paired bytes and add/update/remove identity with
  caller slot keys retained.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --test editor_frame`:
  independent paired fixture, absence/loading/empty frame, every truncation,
  trailing data and invalid gap/label payloads.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --test editor_frame`:
  two atomic admission tests, including descendant-only wrapper mutation, loading
  slot mismatch, non-password reveal, wrong owner kind, frame removal, stable and
  changing policy stamps, retained-byte release and multiline clear rejection.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib editor_frame_view`:
  retained editor, native undoable clear, stale draft/policy rejection, loading
  busy state with typing still enabled, composition guard, single mounting of
  slots, input/text-area width geometry and pointer clear restoring field focus.

The first geometry assertion compared physical AX coordinates to logical GPUI
coordinates. It failed and was corrected to apply the test window's scale factor;
that failure did not establish an application layout defect. A subsequent stronger
height assertion did reproduce a real frame defect: a 120-pixel text area used only
26 pixels of editing height. Stretching the multiline middle container repaired it.
The regression now checks fixed height and automatic growth from three to five
lines. The full native library suite passes **534 tests**, with two existing skips
(`--features native-image-tests --lib`). Full OCaml tests, formatting and the gallery
build pass with `./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.
The catalog source audit and `git diff --check` also pass; this source audit is not
functional acceptance. Strict Clippy passes with
`./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol --features gpuio-native/native-image-tests --all-targets -- -D warnings`.
The full protocol suite (`cargo test -j2 -p gpuio-protocol --tests`) passes, as do
17 native admission checks selected with `--test editor --test editor_frame
--test editor_menu --test menus --test choice_picker` and native-image-tests.
Rust formatting also passes. Clippy's first run found a redundant numeric cast
in the new test; it was removed before the passing final run.

The public password gallery now uses leading content, native clear/loading and
application-controlled reveal alongside its existing edit menu. The gallery builds. Visual/physical checks,
installed-consumer runtime and required Linux checks remain to be recorded. The black-window startup issue is separate and
unresolved.

Exact logs and process checkpoints are local to
`scratch/agents/root-20260929-m7-resumed/OCH-41-input-frame.md`; scratch is not a
build input. Full OCH-41 catalog and OCH-17 release acceptance remain open.
