# Component Studio

A native public-API gallery under implementation for OCH-41. The first sections
cover presentation, selection/actions, native text editing, numeric/range inputs,
verification codes, rating, dates/colors, overlays, navigation/layout and commands/
feedback, carousel/sidebar/history, managed collections, documents and runtime/
window previews, canvas, images/icons, charts, plus native motion sequences, shared clocks, responsive layouts, native extensions and input/transfers. Additional v1
families and the complete coverage ledger are still being integrated; this is
not the completed milestone 07 release.

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the sidebar to switch previews, the light/dark button for appearance, and the
size button for compact/comfortable/large logical sizing. This sizing is not OS
DPI emulation. Each new window has independent navigation, appearance and editor
state. Leaving a page unmounts its native editor leases; appearance/size changes
on the current page preserve native text and selection. Preview model state is
local to its Bonsai branch and survives page changes. Transient modal, chooser and
notification state is cleared on departure. At most four windows are opened by the gallery.

The `--background` launch option avoids requesting focus. macOS accessibility
may not expose an inactive background app's window until activation; interactive
keyboard acceptance therefore launches normally. The native test owns and reaps
its child, including on failure:

```sh
python3 scripts/test_gallery.py --images scratch/gallery-images
```

Use `--section core`, `pickers`, `overlays`, `navigation`, `feedback`, `journeys`,
`collections`, `documents`, `canvas`, `assets`, `charts`, `motion`, `responsive`, `extensions`, `input` or `runtime` for focused iteration;
the default `all` exercises all currently integrated acceptance stages in one
application lifetime. Pickers verify draft cancel/apply and focus restoration;
overlays exercise dialog/drawer/confirmation/popover dismissal and focus; workspace
checks verify retained tab text, hidden editor semantics, accordion and pagination.
The journeys page covers carousel, sidebar and navigation history separately.
Feedback checks shared command buttons/menus/shortcuts, chooser selection, native
toast expiry, close and departure cleanup. Core checks include form error updates
that preserve typed input, avatar semantics and loading-preview controls.
Collections use 1,000 loaded records with bounded active rows/cells. Their internal
preview panels retain native state; leaving the whole page unmounts them.
Documents acquire a fresh child scope per page visit and release registrations on
departure. The runtime page samples public registration counts on demand and can
open a native file picker without reading the selected file.

The canvas page uses a validated pure scene model, with native selection, movement,
activation, viewport commands and hide/disabled controls. The paper stage remains
light in both themes. Hiding retains the scene; leaving releases it and starts
a fresh scene on return. `--trace-canvas` on the app or native test prints public
events, publication readiness and observed window activation for debugging.
The image page registers four in-memory fixtures in its page scope, exercises
SVG/raster fit and native decode failure/recovery, and composes meaningful icons
with decorative button slots. Registration success is distinct from decode success.

Charts share typed example datasets with the standalone Chart Studio. All seven
families and mixed layers support native selection and original-data browsing,
with horizontal, disabled and data-update controls. Each page visit acquires one
scoped registration; leaving releases it. Choices survive a revisit while source
phase and selection start fresh. The original-data table includes node records
for Sankey flows as well as their numerical edge values.

Motion demonstrates interrupted targets, tween/spring sequences, playback controls,
reverse/restart and dynamically joined repeating members. System/Reduced/Full is an
application-wide preference shared by all gallery windows. Leaving the motion page
pauses its sequence and stops its repeating preview. The native test measures
intermediate geometry, interruption, held playback, cancellation, reduced endpoints
and shared phase; there is no OCaml animation-frame timer. `--trace-motion` on
the app or test records semantic sequence batches for diagnosing acceptance runs.

Responsive layouts demonstrate first-match width/height rules and exact logical
breakpoint boundaries. Each branch keeps its own draft and counter while hidden;
only the selected presentation is accessible. The wide branch places editing and
saving side by side. Selection feedback reports the size at the last painted
branch change, not every resize. Leaving the page disposes all branch editors;
returning preserves chosen size and counts but acquires fresh native drafts.

Native extensions consume the independently packaged counter's OCaml library and
reuse `gpuio_counter_backend` from the extension consumer. The app verifies the
linked schema catalog before opening a window. Properties, sequenced commands,
input disabling, retained hiding and generation reset use only public APIs. One
command can be pending; matching acknowledgements update the observed model.
Departure clears pending commands and advances the model generation. The test
enables the package's opt-in lifecycle trace and checks exact unmount/component/
callback-value release, as well as command execution without replay.

Input and transfers demonstrate captured pointer sizing, cancellation and keyboard
alternatives, plus typed text/card drag sources and an inbox for text/custom/file
metadata. Native configuration decides acceptance. File paths are counted without
opening files. The app retains only latest notices, width and a saturated receipt
count; departure clears transient gesture feedback. `--trace-input` records bounded
per-gesture source/target transitions (not every move). The native test enables it
to verify matching gesture identities and exactly two accepted native drops.
This page does not imply passive hover or arbitrary key-event callback parity.

The document AX repair now exposes actual Markdown body text, read-only source/code
and inline links with keyboard activation. Rich/image links, selected-text/range,
heading-level, table and broader screen-reader behavior remain open in the
[document evidence ledger](../../docs/evidence/document-accessibility-och17.md).

The initial macOS test covers semantic navigation, button actions, OS typing and
submission, theme/size changes without resetting the editor, independent windows,
repeated page unmount/remount and window shutdown. It is not an OS IME or complete
catalog qualification claim. See the [design contract](../../docs/design/component-gallery.md)
and [catalog inputs](../../docs/catalog/README.md).
