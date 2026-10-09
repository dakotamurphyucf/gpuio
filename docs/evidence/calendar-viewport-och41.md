# OCH-41 calendar viewport validation

2026-10-02, branch `milestone-07-gallery-release`, base `83eb87e` plus uncommitted
milestone work, macOS arm64, repository stock OCaml 5.3/Bonsai v0.17 and pinned
Rust/GPUI. This is local implementation evidence, not a published release.

Independent asynchronous logical pane observations now reach Core, Eio calendar
and date-picker helpers. Op95 and event tag71 preserve primary calendar handlers
and selection revisions. The public gallery follows each calendar's observed grid
dates for sparse date badges and headings. See [contract](../design/calendar-viewport.md).

Three Core expect tests cover bounded, unique civil grid dates, presentation
changes, independent observer generations, accepted callback updates, malformed/
old/future/duplicate observations, unmount/remount/close, the retained primary
selection callback and paired wire fixtures. A Bonsai driver regression receives
viewport observations during pending picker confirmation, accepts its unchanged
guard and rejects late observations from the previous popup session.

Two native TestPlatform tests use production owners and cover:

- Initial observation ordering, cursor movement into another already displayed
  pane, pane count changes without a selection revision, first weekday, month/
  year presentations, the atomic Changed/Selected pair before viewport publication,
  hidden mounted commands, civil upper bounds and no duplicate idle observations.
- Opt-in/disable/resubscribe, independent handler routing, retained native entity,
  stale owner publication after removal, a new generation's initial sequence and
  explicit sequence exhaustion without wrap.

Two added admission/mailbox tests cover target validation and atomic rejection,
primary handler preservation, bounded adjacent-only coalescing, selection
boundaries, separate handler/revision routes, capacity exhaustion, ordered fault,
window-slot retirement and closed input rejection. The full calendar integration
suite passes **9 tests**. Retained resource assertions include the conservative
additional 128 bytes per calendar.

Testing exposed a real generic mailbox shutdown defect: input could append after
`Stopped`. The generic input lane now rejects callbacks after a queued or drained
terminal marker; a resulting late overload report is also ignored. A native unit
regression covers both direct close and acknowledged shutdown, preserving already
queued output before the terminal marker.

Final local checks pass:

- Full native library: **659 passed, two existing private-D-Bus skips**.
- Full protocol: **333 passed, no skips**, including independent OCaml/Rust
  subscription and three-presentation fixtures and malformed civil boundaries.
- Full OCaml tests, formatting and gallery build; strict all-target Rust lint and
  Rust formatting; source-catalog audit and whitespace checks.
- Fresh installed-library gallery consumer: `INDEPENDENT_EXTENSION_CONSUMER_PASS`,
  `example=gallery`, `run=False`. It stages packages in a fresh scratch prefix and
  compiles copied public example sources without changing an opam switch.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline -j2 -p gpuio-native \
  --features native-image-tests --lib calendar_view::viewport_tests -- --nocapture
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline -j2 -p gpuio-native --test calendar
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline -j2 -p gpuio-native \
  --features native-image-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline -j2 \
  -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/viewport-installed-gallery
python3 scripts/audit_component_catalog.py
git diff --check
```

No physical OS windows were opened. TestPlatform exercises production native
owner/layout behavior but does not qualify physical keyboard, IME, VoiceOver,
GPU rendering or Linux desktop behavior. Current Linux build/unit/consumer gates,
physical macOS qualification and broader milestone 07 release acceptance remain
open. OCH-41 is not complete from these checks alone.
