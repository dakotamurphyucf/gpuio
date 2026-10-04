# OCH-41 native editor viewport — implementation checkpoint

2026-10-01, local macOS arm64, isolated OCaml 5.3 / Bonsai v0.17 and pinned native
dependencies. This is an uncommitted implementation checkpoint, not release or
physical desktop acceptance.

Four initial native TestPlatform checks passed for bounded/clamped scrolling,
acceptance before layout, composition preservation, logical buffer-line ranges,
read-only/disabled behavior, stale native identity and absent pre-layout geometry.
A scroll-during-composition test first reproduced a snap back to zero after a
successful -200-pixel layout. Layout compared an IME endpoint with a differently
saved selection. The adapter's Base patch makes the caret comparison consistent;
masked Unicode and repeated-draw tests cover the related display-offset case.

A fifth test dispatches actual TestPlatform wheel input, verifies that it changes
the live native scroll handle, and checks that a query before the next draw still
returns the previous coherent layout observation. A retained paint-time scroll
offset prevents mixing new wheel position with old row geometry.

Core bounds/codec fixtures, Eio correlation/capacity/close cleanup, delayed
controller metadata replies, native tests, and the public gallery are implemented.
The full native library suite passes **563 tests, with two existing skips**.
Full OCaml tests, formatting and gallery build also pass, including the two Core
expect tests, two Eio request-lifecycle checks and delayed metadata-reply controller
check. These commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` with `-j2`:
`cargo test -p gpuio-native --features native-image-tests --lib --offline` and
`dune build @runtest @fmt examples/gallery/main.exe`. The first full Dune run
found a formatting difference and a test helper named with OCaml's reserved
`effect` keyword; both were corrected before retrying. No test expectation was
blindly promoted.

The canonical Base patch SHA-256 is
`fc481940002bda0a9a57cf62e51263f30b98380eb45bd5abcf3c412148f879d5`.
Hash-verified reconstruction matches all **233 files** exactly (excluding generated
Cargo.lock). All 309 protocol tests pass (`cargo test -j2 -p gpuio-protocol
--offline`), as do `cargo fmt --all --check` and strict Clippy for native/protocol
all targets with `native-image-tests`. Clippy first caught a redundant test
closure; replacing it with the associated constructor made the check pass.

No OS windows were opened by these tests. Physical macOS wheel/IME/keyboard,
accessibility/VoiceOver, visual gallery, installed consumer, performance and
resource acceptance remain separate. Current required Linux automated checks
remain; Linux desktop qualification is deferred OCH-47.

Contract: [native editor viewport commands](../design/editor-viewport.md).
