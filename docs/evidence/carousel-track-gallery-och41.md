# Measured carousel public gallery — OCH-41

2026-10-02, macOS arm64, base `83eb87e` plus uncommitted milestone work.
The Journeys page now includes an “Ideas in motion” preview built entirely from
public Core/Bonsai/Eio APIs. It demonstrates four unequal retained cards, an editor,
per-item styling, horizontal/vertical tracks, viewport resize, reversing item order,
looping, native automatic advancement, immediate/default motion and disabled
navigation. Selection and layout requests reduce in one Bonsai state machine.
The editor controller belongs to the page and survives movement/reordering.

A new runtime expect test exercises `Window_driver` with the public Bonsai view.
Two Next events received before stabilization advance twice; a stale automatic
proposal cannot overwrite that result. Native layout observations update controls
without changing the serialized model revision. Reordering keeps node owners and
selection by ID, rejects old collection geometry and accepts a fresh observation.
Disabled requests and repeated idle cycles produce no transaction; close fences
later callbacks. The test accepts actual reconciler transactions, rather than
calling the model reducer alone.

## Local validation

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/runtime/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace <fresh-local-path>
python3 scripts/audit_component_catalog.py
git diff --check
```

The targeted runtime suite and full OCaml tests/format/gallery build pass.
A fresh installed-package gallery build passes with
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. Catalog and
whitespace audits also pass.
The native implementation's final suite has **705 passing tests and two existing
private-D-Bus skips**; five transport tests and strict lint pass. See the
[visibility evidence](carousel-track-visibility-och41.md) for exact native coverage.
No physical app was launched in this checkpoint.

## Physical walkthrough — not yet executed

Open **Carousels & journeys → Ideas in motion**. Review both Light and Dark
appearance, all preview scales, a narrow window and a second gallery window.

1. Edit “Card draft.” Navigate away and back using the track controls; confirm the
   draft survives and fully clipped cards cannot receive Tab/VoiceOver focus.
   Card controls retain their own keyboard/selection behavior.
2. Focus the measured viewport. Exercise Home/End and horizontal arrows; switch to
   vertical and exercise Up/Down. Confirm the selected-card caption follows the
   accepted selection and visible neighboring buttons remain usable.
3. Reverse cards with the draft present, then resize using Compact viewport and
   window resizing. Check stable draft/selection identity, no jumping content and
   no stray popup/focus outside a visible card.
4. Exercise background dragging, trackpad and line-wheel navigation on both axes,
   including cancellation and edges. Confirm the page can scroll at the appropriate
   vertical boundary and native child text input retains its gestures.
5. Enable Loop cards in both viewport sizes. Inspect continuous wrapping where
   geometry allows it and immediate boundary fallback where it does not. Toggle
   animation and the application's reduced-motion preference.
6. Enable automatic advancement; move pointer/focus outside the group and wait
   for a full four-second interval after settled paint. Group hover/focus, inactive
   window, disabled navigation and reduced motion must pause it. Re-enable and
   verify a fresh full interval, with no burst of missed advances.
7. Leave the page and close the second window during movement. Verify editor,
   timers and native owners retire, and an idle surviving window stays quiet.

This walkthrough defines pending checks, not physical acceptance evidence.
VoiceOver, real IME/keyboard, hardware paint/performance and broader release
qualification remain open. Linux desktop qualification stays deferred under the
platform release policy; required Linux automation remains separate.
