# OCH-41 bound editor menu checkpoint

Local dirty worktree, macOS arm64, 2026-10-01. Implementation and verification are
in progress. No commit, hosted CI, OS window or release acceptance is claimed.
See the [contract](../design/editor-menu.md).

Run commands through the isolated repository environment with `GPUIO_JOBS=2`.

- `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol -p gpuio-native --features gpuio-native/native-image-tests --test editor_menu`
  passes both independent codec and admission tests. The fixture records
  presentation tag 4 in existing SetMenu; truncation/trailing/unknown-tag and
  menu-cardinality failures are rejected. Admission rejects non-editor children
  and callback commands, including atomic rollback of a changed registry.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib editor_menu_tests`
  passes the initial expanded TestPlatform scenario: exact two-editor targeting,
  current password/read-only policy, deferred password/removal rejection,
  right-click, Shift-F10/Escape, focus/selection restoration and composition
  blocking. The final full native library run also passes Cut/Paste/Select-all,
  empty-clipboard availability and directed-selection checks: **532 tests pass**,
  with two existing skips (`--features native-image-tests --lib`).
- `./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`
  passes on the final implementation, including four new Core expectations for
  paired bytes, label/child validation, config updates and keyed reordering.
- Strict Clippy passes: `./scripts/gpuio exec cargo clippy -j2 -p gpuio-native
  -p gpuio-protocol --features gpuio-native/native-image-tests --all-targets --
  -D warnings`. Catalog source audit and `git diff --check` pass.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --tests` passes the full
  protocol suite, including legacy fixtures. Native integration checks with
  `--test menus --test editor_menu` pass all three existing/new admission tests.

TestPlatform clipboard and simulated platform events are not actual macOS
clipboard/keyboard/IME or accessibility acceptance. Actual visual/AX/VoiceOver,
installed-consumer and final Linux build/unit/consumer checks remain required.
The separate black-window issue remains unresolved.

Local per-ticket notes and exact handles/logs are in
`scratch/agents/root-20260929-m7-resumed/OCH-41-editor-menu.md`. Scratch files are
not build inputs. Publication and Linear completion remain outstanding.
