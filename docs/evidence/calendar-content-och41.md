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
