# Component Studio

A native public-API gallery under implementation for OCH-41. The first sections
cover presentation, selection/actions, native text editing, numeric/range inputs,
verification codes, rating, dates/colors, overlays, navigation/layout and commands/
feedback. Additional v1
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

Use `--section core`, `pickers`, `overlays`, `navigation` or `feedback` for focused iteration;
the default `all` exercises all currently integrated acceptance stages in one
application lifetime. Pickers verify draft cancel/apply and focus restoration;
overlays exercise dialog/drawer/confirmation/popover dismissal and focus; workspace
checks verify retained tab text, hidden editor semantics, accordion and pagination.
This initial navigation preview does not yet cover carousel, sidebar or stack.
Feedback checks shared command buttons/menus/shortcuts, chooser selection, native
toast expiry, close and departure cleanup. Core checks include form error updates
that preserve typed input, avatar semantics and loading-preview controls.

The initial macOS test covers semantic navigation, button actions, OS typing and
submission, theme/size changes without resetting the editor, independent windows,
repeated page unmount/remount and window shutdown. It is not an OS IME or complete
catalog qualification claim. See the [design contract](../../docs/design/component-gallery.md)
and [catalog inputs](../../docs/catalog/README.md).
