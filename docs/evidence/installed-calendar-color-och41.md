# Installed calendar panes and color previews — OCH-41

2026-10-05, macOS 14.5 arm64 / M1 Max / built-in display. Two expanded public
gallery walkthroughs pass against the installed application at
`71bd02e44666a86945d642cb14a8f107db2869d7`, executable SHA-256
`d4ad6a43bd258ea414f02891965de3a54c54f8a333b2b9fbdc0b8e39220a3402`.
Its staged consumer build is recorded in [the picker checkpoint](installed-picker-presets-och41.md).
This slice changes drivers and evidence only; no application, protocol, native
adapter or dependency code changes. VoiceOver remains on owner hold and untouched.

## Color palette: 27 GPU cases and native keys

The real pointer hovers Iris and Coral and leaves the palette under both Light
and Dark at Comfortable, Large and Compact sizes. Each of these 24 cases checks
a 3×3 screenshot pixel patch inside the preview against the declared color,
with a six-level RGB tolerance. The preview returns to the actual Mint color
on leave. Featured and normal swatch bounds match the scaled 36/28-point
appearance, and sampled palette positions do not shift when hover changes.

The retained hex editor contains an actual keyboard-entered invalid draft `1`.
Every hover case preserves that draft, the native owner, actual group color
`#89DDC9`, and Mint/Iris/Coral checked states. The passive preview caption is
not independently exposed as static text. The Light/Large Iris capture was
inspected: its preview caption reads `#A3B5FF` while the focused field still
contains `1`. Caption spelling is visual evidence; the automated pixel assertion
checks the swatch color rather than OCR.

After all theme/scale changes, keyboard focus remains on the editor. Typing `2`
produces `12`, proving the insertion point remains at the end. Undo restores `1`,
then the initial color spelling; redo restores `1`. These are real keyboard
operations, not fabricated history observations.

Three more pixel cases check an Iris hover, read-only entry while the pointer
stays over Iris, and attempted Coral hover while read-only. Read-only clears
the preview, restores canonical draft text and retires discarded edit history,
as specified by the existing color-input contract. Undo after returning to
editable mode cannot resurrect the cancelled draft.

Actual Tab, Shift+Tab, Left/Right, Home and End exercise the Palette/HSLA roving
tab stop. HSLA exposes Hue; returning with Shift+Tab reaches the hex editor
without stopping on the inactive tab. The same editor/value survives panel
changes. Leaving the page removes the owner; remount creates a new owner with
the initial value. Two complete runs pass, including the final driver with
explicit initial page/size setup. Both applications close and are reaped.

## Calendar: 11 viewport cases and a cross-month range

Real Return/Right/Return input selects September 30–October 1, 2026. Both dates
are checked, neighboring dates are not, and the cursor crossing into October
does not shift the initial September–October two-pane viewport.

The driver compares every exposed day target and the application's viewport
observation with an independent Python civil-date oracle: six Monday-starting
weeks for each displayed month, deduplicated across panes. It checks all dates,
not just a count or a few samples. The 1/2/3/12-pane configurations expose
42/70/98/371 unique date targets in these cases, with no omissions or duplicates.
The calendar owner, range selection and October day target survive count and
Light/Dark changes. Shrinking to one month follows the October cursor.

Month/year selection modes publish an empty day-grid prefetch set. Returning to
October Days restores the expected 70-date set without changing the range.
Page retirement removes the calendar; remount creates a new owner and empty
selection. The complete run exits 0 with 11 viewport samples and both success
markers. The Light three-pane capture shows the third month wrapping below the
first two. Twelve panes exceed the window: captures show their beginning only;
the logical target assertions cover all panes, not full-screen paint for each.

## Preserved harness failures and scope limits

The archive retains the initial failures and their corrections:

- This compound hex field does not expose `AXSelectedTextRange`; insertion-point
  preservation is instead tested through typing and undo. No adapter was changed.
- Revealing only the editor left swatches offscreen; the ownership guard rejected
  the attempted pointer target. Shrinking the page could conversely clip the
  editor while leaving swatches visible. The driver now reveals both before
  sampling. Pointer guards and pixel tolerances were not weakened.
- A draft-retention assertion initially crossed a read-only transition. Existing
  policy deliberately cancels active editing there. Hover/history retention is
  tested before that boundary, and canonical text/history retirement afterward.
- The calendar initially required September 30's target identity to survive
  removal of its September pane. Day targets are structurally keyed under their
  displayed month; October's padding is a different target. The final test
  requires stable calendar ownership/selection and identity within retained day
  panes, not identity across structural movement or retired Days presentation.

The final selectors are included in `all`:

```sh
python3 scripts/test_gallery.py --section color-preview --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section calendar-viewport --executable <installed-main.exe> --images <output>
```

Python compilation, structural catalog audit and whitespace checks pass.
[The archive](installed-calendar-color-och41/local-validation.tar.gz) and
[manifest](installed-calendar-color-och41/manifest.json) contain exact commands,
terminal results, sample JSON, captures, failures and final drivers.
These checks do not qualify actual marked-text composition, arbitrary selection
ranges, every palette family/channel gesture, all date constraints/locales/civil
boundaries, full calendar pixel geometry at every scale, measured resources or
performance. Native TestPlatform evidence covers additional contracts separately.
Final-source hosted CI, clean-machine distribution and release acceptance remain
open. Linux desktop stays deferred to OCH-47; no Linux GUI claim is made.
