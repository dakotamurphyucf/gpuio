# Component Studio

A native public-API gallery under implementation for OCH-41. The first sections
cover presentation, selection/actions, native text editing, numeric/range inputs,
verification codes, rating, dates/colors, overlays, navigation/layout and commands/
feedback, carousel/sidebar/history, managed collections, documents and runtime/
window previews, plus native motion sequences and shared clocks. Additional v1
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
`collections`, `documents`, `motion` or `runtime` for focused iteration;
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

Motion demonstrates interrupted targets, tween/spring sequences, playback controls,
reverse/restart and dynamically joined repeating members. System/Reduced/Full is an
application-wide preference shared by all gallery windows. Leaving the motion page
pauses its sequence and stops its repeating preview. The native test measures
intermediate geometry, interruption, held playback, cancellation, reduced endpoints
and shared phase; there is no OCaml animation-frame timer.

The document AX repair now exposes actual Markdown body text, read-only source/code
and inline links with keyboard activation. Rich/image links, selected-text/range,
heading-level, table and broader screen-reader behavior remain open in the
[document evidence ledger](../../docs/evidence/document-accessibility-och17.md).

The initial macOS test covers semantic navigation, button actions, OS typing and
submission, theme/size changes without resetting the editor, independent windows,
repeated page unmount/remount and window shutdown. It is not an OS IME or complete
catalog qualification claim. See the [design contract](../../docs/design/component-gallery.md)
and [catalog inputs](../../docs/catalog/README.md).
