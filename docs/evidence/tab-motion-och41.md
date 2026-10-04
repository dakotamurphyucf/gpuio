# Native tab motion — OCH-41

2026-10-02, macOS arm64, base `83eb87e` plus uncommitted milestone work.
Core/Bonsai tab constructors accept optional `Tab_bar.Motion`; omission keeps
static presentation. Pill, Segmented and Underline indicators use native left/
width springs, with GPUIO defaults 400/40/1, epsilon 0.01, maximum two seconds.
Newly selected enabled Pill content fades inherited foreground over 200 ms with
cubic easing. The Navigation gallery enables this on its closable workspace tabs
and exposes **Animate tab selection**. Tab/Outline remain static.

The owner retains only bounded geometry, spring history and a selected ID. A
frame measures the first and selected native tab boxes; coordinates relative to
the first prevent scrolling or ancestor translation from retargeting motion.
First layout adopts the target. Selection, resize and reorder retarget from the
last painted position and velocity. Policy/variant changes, reduced motion and
inactive windows settle immediately. Missing selection, hidden/zero/clipped
owners, metadata reset, unmount and window close retire motion. Only active
paint schedules a next frame; no permanent timer or OCaml layout callback exists.

Explicit descendant foreground colors retain precedence. Per-tab custom fills
and borders paint on their own targets above the traveling default indicator.
Its opacity follows the selected target, including disabled-item half-opacity.
A paint-only theme change preserves the geometry trajectory. Very large valid
layout coordinates outside the generic spring domain adopt exact target values.
See the [contract](../design/tab-motion.md).

## Local evidence

- OCaml expect tests check duration rejection/rounding, exact independent Op103
  bytes and all three public tab constructors. Enable/change/reset emits one
  metadata operation, preserves native IDs and gives a no-op for identical values.
- Independent Rust codec tests check default/zero/maximum duration, every
  truncation, trailing bytes and malformed spring/duration values. Atomic native
  admission rejects non-tab owners and bad values, charges 4096 bytes per enabled
  owner, and releases the reservation on reset/removal.
- Three pure native state tests check initial paint, interrupted velocity,
  geometry changes, bounded settlement, noncommitting color previews, changed
  policy, zero duration, large-coordinate fallback and owner reservation size.
- Seven native tests exercise production host/radio layout and scene paint on
  TestPlatform: one indicator without a duplicate default selected fill;
  midpoint/end/interruption and width changes; scroll translation, real native
  wheel event dispatch and Choice-ID reorder; all three moving variants;
  reduced/inactive/disabled/missing/hidden/zero-layout behavior; idle frame
  counts, metadata removal, unmount and window-close retirement; custom target
  fill order, theme updates and disabled opacity.
- The foreground test uses a native descendant paint probe through the same
  production radio renderer. It observes blue → midpoint purple → red inherited
  color and an explicit green descendant override. TestPlatform produced no font
  glyph sprites in this setup, so this is native inheritance/scene evidence,
  **not** a GPU font-raster or physical desktop appearance claim.

A failing native test found that the host's deferred paint sweep omitted windows
containing only tab motion. Adding tab owners to that eligibility condition fixed
hidden/missing-selection retirement. The disabled-opacity check also caught and
fixed a discrepancy between the selected target and its traveling indicator.

Validation commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib`: **725 passed, 0 failed, 2 existing private-D-Bus skips**.
- `cargo test --offline --locked -j2 -p gpuio-protocol`: **346 passed, no skips**.
- `cargo test --offline --locked -j2 -p gpuio-native --test tab_motion --test choice_menu_icons --test choice_menu --test tab_trailing --test tab_viewport --test tab_content --test tab_appearance --test control_labels`: **15 passed**.
- `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`: **pass**.
- Initial full `dune build -j2 @runtest examples/gallery/main.exe`: **pass**.

- Final `dune build -j2 @runtest @fmt examples/gallery/main.exe`: **pass**.
- `cargo fmt --all -- --check`: **pass**.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
  --workspace scratch/agents/root-20260929-m7-resumed/tab-motion-installed-gallery`:
  **fresh installed consumer pass, `run=False`**.
- Catalog structural audit and `git diff --check`: **pass**; the catalog audit
  does not establish component behavior or release acceptance.

Local logs and ticket-specific notes are under
`scratch/agents/root-20260929-m7-resumed/` with the `tab-motion-` prefix.

This does not complete OCH-41 or milestone 07. Flat split-group functionality and
macOS physical GPU/input/VoiceOver, resource/performance, distribution/release and
required Linux automation acceptance remain separate requirements. No physical
OS windows were opened for this checkpoint; no Linux runtime, hosted CI, release
publication or Linear-write result is inferred from these local tests.
