# Text-area search observations — OCH-41

Local implementation checkpoint, 2026-10-01, on macOS using the isolated
repository toolchain. This is automated behavior evidence, not physical desktop,
IME, VoiceOver or release acceptance. No vendor change is required by this work.

Appended event70 delivers bounded native search metadata through the existing
window/node/handler route. Core exposes `Text_input.Event.Search_changed`; Eio
retains it as `Text_input.search_snapshot`. The draft remains native-owned.
Subscriptions publish initial metadata after text, deduplicate unchanged state,
and stop with the editor. Disable emits closed state. Adjacent same-owner,
same-view, nonregressing observations coalesce without crossing other events.

An independent bin-prot fixture checks the appended tag and all payload fields in
Rust and OCaml. Core tests reject invalid metadata, truncated/trailing envelopes,
foreign node/window/handler leases, future/negative view revisions and open events
after opt-out. Closed observations remain routable during opt-out; window close
drops them. Eio's actual Bonsai driver test receives the new event, then exercises
delayed replacement replies after newer typing and a new native-controller lease.
Reducer tests cover independent text/search revision ordering and metadata-only
updates without draft mutation.

The native retained-view test dispatches Cmd-F (Ctrl-F on non-macOS builds) on
GPUI's TestPlatform and verifies the resulting observation. Native query/navigation
changes, repeated unchanged notifications, disable and retained-entity activity
after removal are also checked. This exercises the native action route without
an OS window; it does not establish physical keyboard or query-bar focus behavior.
Mailbox tests cover growing queries, burst coalescing, ordering barriers, older
stamps, count saturation, output ownership and empty-queue accounting.

Three regression cases were reproduced before their fixes:

- A delayed replacement reply could install a retired editor lease after a new
  native mount observation. Replacement replies now use the controller's guarded
  reply path. The test explicitly models a new mount at the controller boundary.
- Search-result payloads were absent from mailbox byte accounting. Eight replies
  containing maximum-size text could serialize beyond the 1 MiB frame limit.
  Charging text and query bytes now splits them into bounded batches while
  delivering all replies and releasing response reservations.
- A delayed equal-stamp search reply could restore `can_replace` after a newer
  native read-only observation. Existing metadata now wins reply ties; ordered
  native observations can still update capability fields with the same stamp.

Validation commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`, with `-j2`:

- `cargo test -p gpuio-native --features native-image-tests --lib --offline`:
  **576 passed, two existing skips**.
- `cargo test -p gpuio-protocol --offline`: **311 passed**, including the paired
  observation fixture.
- `dune build @runtest @fmt examples/gallery/main.exe`: passed again after the
  final equal-stamp reply fix, including the new native-event Bonsai driver test.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 scripts/audit_component_catalog.py`: passed (structural source mapping,
  not behavioral acceptance). `git diff --check` also passed.

The native and protocol runs completed before the final OCaml-only equal-stamp
fix; Rust implementation did not change afterward. The Eio regression failed
with the earlier reducer, then passed with the fix. No new OS window was opened
for these checks. Repository notes retain the commands, result logs and exact
local test sequence; the changes are not yet a committed/reviewed release.

Remaining work includes the reusable search bar and gallery, query focus and
restoration, complete Find/F3/Escape presentation integration, physical macOS
input/accessibility checks and measured resources/performance. Linux automated
checks remain required; physical Linux desktop qualification remains deferred.
