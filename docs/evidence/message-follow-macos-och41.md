# Message-follow macOS walkthrough — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max, based on `501430a5`.
This follow-up repairs an OCaml example sizing defect and adds focused physical
input/anchor checks. OCH-41 and OCH-17 remain open.

## Repair and observed behavior

The initial run passed Light/Comfortable, then measured the Large follow action
at 51 pixels high inside the recipe's clipped 48-pixel slot. The capture shows
its top/bottom border clipped. `collections_page.ml` now uses `follow_button`
with height 44, line height 20 and horizontal padding 12, while preserving
palette-scaled typography and hover colors. The repaired Large capture was
visually inspected; the full button border fits. The library, Rust host,
protocol and scroll controller are unchanged. This is not an explanation of
the separately reported high-speed benchmark row overlap.

Final repository and installed-consumer runs each pass Light/Dark ×
Comfortable/Large/Compact. Full application motion is used for Comfortable and
Large, Reduced for Compact; this is not a full motion/size cross-product.
Each case checks:

- The visible action fits its 48-pixel slot and lies within the transcript.
- Toggling jump/fade/animation presentation preserves list bounds and the first
  exposed reading anchor's text/bounds, within one pixel after settlement.
- Native Space activates the focused action. Following hides it from AX;
  growing the latest response retains following state.
- An actual OS line-wheel event at the now-hidden slot moves the transcript
  into history and makes the action available again.
- Prepending history, appending a message and growing the latest response while
  reading history preserve that sampled reading anchor and viewport geometry.
- Pointer activation returns to following; the Bonsai mounted-row caption
  remains nonzero and at most 24. Leaving the page retires the native list/action.

The driver uses guarded foreground keys, OS pointer/wheel events and AX actions/
observations. It changes no clipboard or OS settings. Both owned applications
close and are reaped normally. Sampled settled bounds and the active-row caption
are not per-frame overlap, physical frame timing, hardware trackpad, allocation/
retained-memory or VoiceOver acceptance. Live semantics remain explicitly Off.

## Reproduction and evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @test/gallery/runtest @fmt
python3 scripts/test_gallery.py --section message-follow --images scratch/follow-root
python3 scripts/test_gallery.py --section message-follow --executable <installed-main.exe> --images scratch/follow-installed
python3 -m py_compile scripts/gallery_message_follow.py scripts/test_gallery.py
ruff check scripts/gallery_message_follow.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
```

The native runs use 240-second SIGALRM watchdogs that raise through normal
harness cleanup. `follow-native-001` retains the failing pre-fix evidence;
`follow-native-002` and `follow-installed-001` both finish six successful cases
and `GPUIO_GALLERY_AX_OK`. The installed example was rebuilt with the changed
`collections_page.ml` against the existing horizontal-consumer installed prefix.
Libraries were reused, not newly installed this turn; both catalog schemas pass.

The [verified archive](message-follow-macos-och41/reports.tar.gz) retains all
attempt logs/reports/captures plus build logs. Its
[manifest](message-follow-macos-och41/manifest.json) records exact binary hashes,
source base, platform and the hashes/sizes of 23 artifacts (7,233,216 bytes).
Focused gallery expect tests, formatting and builds pass. Python syntax and
focused Ruff pass; example inventory remains 432 sources/268 reviewed groups.
The new four-minute CI step is pending: Foundation 37789987337 predates this
change. Required Linux nongraphical and other release checks remain intact.
