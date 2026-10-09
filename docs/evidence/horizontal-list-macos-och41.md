# Horizontal-card desktop qualification — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max; source based on `fa2fd39b`.
This public OCaml/Bonsai example starts with 10,000 unequal-size cards and a
16-row active budget. Its native library implementation is unchanged by this
follow-up. OCH-41 and OCH-17 remain open.

## Clipped control and example repair

The initial physical inspection of First card found its reaction button outside
the row's clipped bounds. At Comfortable size the first row was 180px wide and
260px high: its bottom was y967.5, while the button began at y983 and ended at
y1047. The screenshot confirms the missing action and truncated paragraph. The
probe itself only gathered evidence; its success marker is not visual acceptance.

The example now uses base extents 260/284/308/332 and a 360px cross-viewport height,
with both main-axis extent and viewport height scaled by the palette size factor.
Native measurement estimates likewise scale from 300px. This provides space for
the existing content and actions in both axes, without weakening clipping,
removing controls or changing library virtualization. The source still grows
extents by 32 up to 500, has at most 10,100 records, and keeps a 16-row active budget.

## Desktop behavior

The focused `--section horizontal-list` runner checks Light/Dark ×
Comfortable/Large/Compact, each in both axes, in repository and freshly installed
examples. These are application text/layout sizes, not display-scale transitions.

- Actual OS pointer activates the first horizontal card's reaction. Foreground
  Space activates its vertical counterpart. The Bonsai reaction count changes
  through ordinary native input; the test does not replace model state.
- Every exposed card's reaction rectangle fits its card. The tested first action
  also fits the list viewport, so semantic activation alone cannot hide an
  unreachable control. Horizontal growth changes measured width and retains the
  reaction count.
- Eight discrete OS line-wheel steps per axis exercise real AppKit delivery.
  Sampled adjacent row rectangles are contiguous within 1px, and surviving rows
  move in the expected direction. Both-axis changes preserve the surviving first
  row's nonzero reaction count. Evicted transient state has no persistence promise.
- The public caption reports 3–4 active Bonsai rows after each case, within the
  configured 16. Exposed AX rows are checked separately and are not used as a
  substitute for the Bonsai count. This is not an RSS or full-history benchmark.
- A separate sequence reveals key 5000, reverses order, prepends and appends while
  reading, preserving the observed anchor key and its x-position within 1px.
  Follow latest reaches the newly appended key 10000; another append reaches 10001.
  Leaving the page retires the list from the accessibility tree.

The final driver waits for asynchronous page retirement. An earlier installed
run passed all six input/geometry cases and navigation operations, then queried
absence immediately after navigation and failed. The final check uses the
existing bounded `wait_absent`; the failed attempt remains in the archive.
The earlier root run's immediate-absence check is superseded by the final runs.

The Dark/Large repaired capture was inspected: both visible cards have readable
actions contained by their panels. Captures and AX geometry are sampled evidence,
not proof about every presented frame. No claim is made for hardware trackpads,
VoiceOver, IME, full scrollbar policy, frame timing, complete history traversal or
Linux graphical acceptance. This example sizing defect does not establish the
cause of the separately reported loaded-list benchmark jitter/overlap.

## Commands and evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/horizontal-consumer
python3 scripts/test_gallery.py --section horizontal-list --images scratch/horizontal-root
python3 scripts/test_gallery.py --section horizontal-list --executable <installed-main.exe> --images scratch/horizontal-installed
```

The fresh installed consumer passes its independent counter/document-profile
catalog checks. Final GUI commands use 240-second Python SIGALRM watchdogs that
raise through normal harness cleanup. Each runs one owned application and closes
it afterward. The tests do not use the clipboard or change OS settings.

The [archive](horizontal-list-macos-och41/reports.tar.gz) retains the original
probe/tree/capture, failed and passing attempts, build logs, source snapshots and
lint comparison. Its [manifest](horizontal-list-macos-och41/manifest.json) records
executable identities, completion summaries and SHA256/size for each verified
entry. Final reports are `horizontal-native-003` and `horizontal-installed-002`.

Python syntax, new-module Ruff, unchanged legacy-driver diagnostics, actionlint,
formatting/build and example/catalog inventories pass. A four-minute Foundation
step is added. The live Foundation 37789987337 predates this addition and source
repair; no current-source hosted or Linux acceptance is claimed here.
