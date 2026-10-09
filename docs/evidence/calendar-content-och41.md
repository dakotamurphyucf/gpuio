# Calendar content — OCH-41

2026-10-02, local macOS worktree based on `83eb87e`. The typed content path,
native renderer, managed picker composition and public gallery are implemented.
This is local codec, driver and TestPlatform evidence; physical macOS acceptance
remains open.

`Calendar.Slot` validates civil dates/years and keeps distinct identities for
navigation controls, day/month/year choices and month-qualified headings.
`View.Calendar_content` sorts and validates passive content before accepting it.
The paired metadata implementations enforce 1024 canonically ordered unique slots,
1024 UTF-8 bytes per description and 65536 description bytes overall. Native
decoding checks aggregate allowance before allocating each string.

The independent 56-byte metadata fixture covers all 11 slot tags. The independent
70-byte Op89 envelope covers setting and clearing content. OCaml and Rust agree
on the bytes. Boundary tests cover malformed values, order/duplicates, UTF-8,
allocation limits, invalid tags, every truncation and trailing bytes.

Core constructor tests reject interactive callbacks, duplicate/excess slots,
excess aggregate nodes and excessive depth including generated wrappers. Public
reconciliation tests show supplied order does not change identity and text-only
updates do not recreate the owner or reset its configuration. The managed picker
driver updates content during an outstanding confirmation, preserving the draft,
native target and completion callback. The matching reply still commits once.

Native admission tests reject malformed wrappers, mismatched content, nested
buttons, callback and selectable-style updates atomically. They verify exact
metadata string accounting and return to baseline after removal.

Two production-host TestPlatform regressions cover:

- Rich date/navigation/month/weekday content rendering once, with a hidden
  decorative accessibility ancestor and preserved native name/description/ID.
- Descendant-only text/style updates preserving range draft, keyboard focus and
  native owner. Mouse and AX selection still target the date; read-only policy
  rejects selection. Switching presentation hides inactive content, and removal
  restores the default native label without remounting the calendar.
- Progress content paint/style updates, stopping frame requests while offscreen,
  resuming the same owner when visible, and releasing resources after removal.

The hide test exposed duplicate rendering through the generic child-layout path.
Calendar wrappers now mount exclusively through their assigned native slots. The
regression asserts exactly one rendered content label and absence in the hidden
presentation; it does not just ignore duplicate accessibility entries.

The public gallery uses arrows, a month heading and live date event badges in
both inline and popup calendars. `Event badges` restores defaults; `Update events`
changes a retained date's count and description. A macOS walkthrough is authored
and syntax-checked in `scripts/gallery_calendar_content.py`, called by
`test_gallery.py --section pickers`; it has not run on a physical desktop.

## Commands and results

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib host::calendar_view::content_tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol \
  --offline --test calendar_content
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
```

All commands above pass. The native library reports **633 passed/two existing
private-D-Bus skips**; the full protocol suite reports **326 passed/no skips**.
The focused admission/codec run passes four tests. Full OCaml tests, formatting
and gallery build pass. Catalog structural audit and diff whitespace checks pass.
No OS windows were opened. Strict Rust Clippy also passes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

A fresh staged installation also builds the independent gallery successfully:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/calendar-content-installed-gallery
```

It reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
This stages the public libraries without changing the opam switch, then builds
copied gallery sources against them and the rebuilt native backend. The path is
local evidence, not a repository dependency. No consumer application was launched.

These checks do not qualify physical macOS input/accessibility/VoiceOver/GPU,
performance/resources, installed-consumer runtime or Linux GUI behavior. Exact viewport
observation is separate from the existing cursor-month snapshot and remains an
open design. OCH-41 and OCH-17 remain incomplete. See the
[content contract](../design/calendar-content.md).

## Public constructor/native admission repair — 2026-10-04

A real macOS 14.5 arm64 run of `test_gallery.py --section pickers` at production
revision `984210e` exited with `Invalid_tree` as soon as Dates & colors mounted.
The native slot validator correctly required structural wrappers without style
declarations. `View.Calendar_content.create` built those wrappers with
`container []`, whose empty base-style map encoded as `[Fields []]` instead of
`[]`. Hand-authored Rust fixtures had used truly empty styles and missed the
public constructor mismatch.

The constructor now uses the same unstyled structural-node pattern as number
and editor frames. Slot identity, custom child styles, ordering and strict
native validation remain intact. `calendar-content-public.hex` is a shared
342-byte transaction: the OCaml test compares actual public reconciliation bytes
against it, and the native test decodes it into a Session, validates the two
slots and their content, and closes the window with zero retained tree bytes.
Temporary diagnostic prints used to isolate the rejected node were removed.

The rebuilt gallery passes the physical picker run: event-content updates keep
the same native day target and selection; event-badge removal/restoration updates
help text; the rich day still selects a range start. Appointment and color
pickers preserve committed values on cancel, apply new values, clear values and
shut down successfully. This is actual AppKit AX interaction on the local desktop;
it does not establish VoiceOver, candidate IME, GPU performance or Linux GUI
acceptance. Evidence: `gallery-pickers-001.log` (original failure),
`picker-native-diagnostic-002.log` (rejected wrapper), and
`gallery-pickers-003.log` (pass), under `scratch/agents/root-20261004-resumed/`.

The local command `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
examples/gallery/main.exe @test/view_api/runtest @fmt` passes after the fix.
The physical command is `python3 scripts/test_gallery.py --section pickers
--images scratch/agents/root-20261004-resumed/pickers-images-003`.

The default-feature native `cargo test --locked --offline -j2 -p gpuio-native
--test calendar_content` passes both the shared public transaction and the
existing atomic shape/passivity/heap-accounting test. Strict Clippy for that
test and `cargo fmt --all --check` also pass through the isolated wrapper.
Logs: `calendar-public-native-001.log`, `calendar-public-clippy-001.log` and
`calendar-public-rustfmt-001.log` in the same session directory.

The broader `dune build -j2 @all @runtest @fmt` also exits zero for this calendar
repair (`calendar-full-dune-001.log`). Later choice-picker accessibility changes
have their own [native and physical evidence](choice-picker-macos-och41.md).
