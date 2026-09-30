# Component Studio

A native public-API gallery under implementation for OCH-41. The first sections
cover presentation, selection/actions, native text editing, numeric/range inputs,
verification codes, rating, dates/colors, overlays, navigation/layout and commands/
feedback, carousel/sidebar/history, managed collections, documents and runtime/
window previews, canvas, images/icons, charts, plus native motion sequences, shared clocks, responsive layouts, native extensions, input/transfers, desktop services, styling details, general input observations and subtree highlighting. Additional v1
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
in-app toast state is cleared on departure. Application-owned OS notification/link state survives page and window changes. At most four windows are opened by the gallery.

The Presentation page's **A little light, in motion** card demonstrates selectable
text-glyph shimmer with explicit application-theme colors. It starts paused. Use
Start/Pause, reverse, one sweep, width and effect toggles; Refresh changes the
source and replays a finished sweep. Animation stays in Rust and respects reduced
motion. Its focused normal-launch and installed-consumer checks pass on macOS:
`python3 scripts/test_gallery.py --section shimmer`. See the exact coverage and
remaining release gates in [the shimmer contract](../../docs/design/text-shimmer.md).

The **Keys with meaning** card uses typed `Shortcut` values and `Presentation.Kbd`
for filled/outline/plain keycaps, macOS/Linux formatting, accessible names and
style overrides. Its explicit registration switch separates displaying a label
from invoking a command. Run `python3 scripts/test_gallery.py --section keyboard-labels`;
it also runs in `core` and `all`.
See the [display contract and remaining native binding-query work](../../docs/design/keyboard-labels.md).

The **Details that stay together** card uses `Presentation.Description_list`
for rich term/value slots, columns/spans, horizontal/vertical cells, four sizes,
borders and separators. Reflow and reorder preserve the embedded draft and
controls. Run `python3 scripts/test_gallery.py --section descriptions`; the check
also runs in `core` and `all`. See [the contract and native/installed-consumer
evidence](../../docs/design/description-lists.md).

The **Room for a conversation** card previews the new typed `Presentation.Bubble`
and `Presentation.Message` compositions: optional slots, Ghost spacing, independent
alignment, native reaction buttons, an editor and bounded streaming Markdown/code.
Run `python3 scripts/test_gallery.py --section chat-composition` for the focused
walkthrough. The **A conversation in motion** card puts these compositions in a
100-message managed transcript with a 12-row bound, explicit reaction spacing,
a scoped streaming document and an outside draft. Use `--section chat-list` for
streaming/history, nested scrolling and cleanup checks. See the exact evidence
and remaining release gates in the [contract](../../docs/design/chat-composition.md).
Both sections are also invoked by `core` and `all`; the expanded combined run
remains a consolidated release check.

The **Small details, useful actions** card uses `Presentation.Tag` for rich
content, semantic/custom palettes, outline, sizes, rounded corners and native
hover refinement. Remove the label to keep a control-only tag, reorder children,
or remove both slots. Run `python3 scripts/test_gallery.py --section tags`; this
check is also included in `core` and `all`.
See the [Tag contract](../../docs/design/presentation-tags.md).

The **A clear next step** card uses `Presentation.Alert` for semantic variants,
Card/Banner layout, optional title/close, icon choices, sizes, style refinement
and narrow-width wrapping. Dismiss with pointer or keyboard, test disabled close,
and restore the alert while its counters remain in caller state. Run the focused
native check with `python3 scripts/test_gallery.py --section alerts`; it also runs
in `core` and `all`.
See the [Alert contract](../../docs/design/presentation-alerts.md).

The **Signals that stay out of the way** card uses `Presentation.Marker` for
Plain/Separator/Border status rows. Toggle busy state, Spinner/Shimmer, custom or
empty icons, typed/rich content and an empty typed label. Typed text shimmers;
rich-only content pulses; the adjacent **Steady** label remains unchanged. The
action retains its counter and focus across loading/layout changes. Style and
compact-width controls demonstrate refinements. Removing the rich slot intentionally
retires its action; adding it back preserves the Bonsai counter. The focused check
is `python3 scripts/test_gallery.py --section markers`; it is also part of `core`
and `all`. See the [Marker contract](../../docs/design/presentation-markers.md).

The Presentation page's **Structure with flexibility** card uses
`Presentation.group_box` with Card/Plain/Filled/Outline variants. Toggle its
header/footer and independent slot styles while keeping the body checkbox state.
The group owns no model or native controller; this example's Bonsai model persists
across page navigation while the native controls unmount. Run its focused native
check with `python3 scripts/test_gallery.py --section groups`.

The **Attachments with a little more to say** card uses
`Presentation.Attachment`: five statuses, horizontal/vertical layouts, four sizes
plus a custom size, optional slots, image decode failure/recovery and separate
whole-card/Save actions. Uploading and processing titles shimmer natively; image
opacity leaves the overlay and controls unchanged. Toggle **More attachments** and
**Constrain attachment group** to try the horizontal strip and independent card
actions. Decode work and registrations belong to the page's resource scope. Run
`python3 scripts/test_gallery.py --section attachments` for its focused macOS
interaction checks; `--section attachment-paint` starts a fresh app for GPU color,
native motion-preference and horizontal/parent scrolling checks. See
[the contract](../../docs/design/presentation-attachments.md)
for ownership and validation limits.

The Styling details page's **Proportions that follow your layout** card uses
`Aspect_ratio` to resize a preview while retaining a child button's count and
focus. Switch square/landscape/portrait proportions, compact/wide frames and
automatic/fixed height. The focused macOS check is
`python3 scripts/test_gallery.py --section aspect-ratio`; both the repository app
and an independently installed consumer pass the
[layout and state checks](../../docs/design/native-aspect-ratio.md).

The Presentation page includes a **Text with context** card using `Label.create`
and `Presentation.styled_label`. It demonstrates one selectable text flow with
muted secondary text, prefix/all-occurrence Unicode match coloring and display
masking. Theme, secondary, match and compact/wide changes preserve native text
identity. Masked values submit only bullets; default accessibility and copy expose
that replacement text. This is display masking, not a password editor.
The focused native check is `python3 scripts/test_gallery.py --section labels`.

The composed-link card offers **Rich link previews**: an image-backed avatar,
an image and a native loading indicator, each sharing its link's single action
and focus target. The scoped SVG registration is reused by the passive previews.
The counter and **Last opened** label show the actual destination; no browser is
opened. Toggle descriptions, decoration, disabled state and Tab policy while
retaining link identity. Leaving the page retires the content and its resources.
Run `python3 scripts/test_gallery.py --section links` for the focused walkthrough.

The Input observations page wraps a retained native editor with public
`View.input_region` subscriptions. Capture/bubble and enable controls update the
observer without replacing the editor. A floating action uses explicit pointer
occlusion while wheel input may pass through. Thirteen saturating counters and three
latest-sample summaries stay bounded; observations reset on page departure.
Disabling observations does not disable the child editor. Native shortcuts and
IME retain ownership of editing; raw key callbacks are not a text-input API.

The `--background` launch option avoids requesting focus. macOS accessibility
may not expose an inactive background app's window until activation; interactive
keyboard acceptance therefore launches normally. The native test owns and reaps
its child, including on failure:

```sh
python3 scripts/test_gallery.py --images scratch/gallery-images
```

Use `--section core`, `status-regions`, `badges`, `styles`, `pickers`, `overlays`, `navigation`, `feedback`, `journeys`,
`collections`, `documents`, `document-links`, `document-images`, `highlighting`, `canvas`, `assets`, `charts`, `motion`, `responsive`, `extensions`, `input`, `observations`, `desktop` or `runtime` for focused iteration;
the default `all` exercises all currently integrated acceptance stages in one
application lifetime. Pickers verify draft cancel/apply and focus restoration;
overlays exercise dialog/drawer/confirmation/popover dismissal and focus; workspace
checks verify retained tab text, hidden editor semantics, accordion and pagination.
The journeys page covers carousel, sidebar and navigation history separately.
Feedback checks shared command buttons/menus/shortcuts, chooser selection, native
toast expiry, close and departure cleanup. Core checks include form error updates
that preserve typed input, avatar semantics and loading-preview controls.
Presentation includes a status bar with individually toggled leading, center and
trailing regions. The focused `status-regions` check (also in `core`/`all`) measures
adaptive center alignment and retained native button identity, and exercises
keyboard actions across all eight slot combinations, both themes and three sizes.
The same page demonstrates count/dot/icon overlay badges with zero hiding,
uncapped accessible descriptions and an independently clickable underlying
button. `--section badges` checks actual anchors, pointer input through badges,
keyboard traversal, retained native identity, both themes, all badge sizes and
scoped SVG cleanup. The decorative icon deliberately has no separate AX node.
Styling details demonstrates clip/end/start text truncation at two widths and all
22 typed cursor choices, plus a live sRGB/Oklab gradient interpolation swatch.
Full text remains available to accessibility. Preview
choices survive page visits; the test verifies configuration, keyboard activation,
geometry and retention, with screenshots for truncation. It does not assert the
physical OS cursor artwork, which may share glyphs between related choices.
Collections use 1,000 loaded records with bounded active rows/cells. Their internal
preview panels retain native state; leaving the whole page unmounts them.
Documents acquire a fresh child scope per page visit and release registrations on
departure. Its Image alternatives mode shows explicit registered raster images,
linked alternatives, decorative images, reference-style Markdown and a safe
missing-image placeholder. Images do not fetch URLs implicitly. The document
checks query actual macOS reading order and named image/link targets, then exercise
focus, activation, collapse and remount. The runtime page samples public registration counts on demand and can
open a native file picker without reading the selected file.

Find & highlight uses `View.highlight_scope` over ordinary/selectable text and a
scoped Markdown document. Its native query editor drives case/whole-word controls;
next/previous select a match color without automatically scrolling to that match.
One word crosses two styled text leaves. An empty nested scope excludes a private
note, while a separate UTF-8 byte range marks `世界` independently. Match counts
come from queued native observations paired with the current configuration.
Appending a paragraph is bounded and idempotent; collapse removes the document's
matches and clamps the selected index. Leaving clears observations/cursor and
releases the document/editor; revisiting creates a fresh notebook/query. Toggle
preferences survive page visits. The focused native test covers query edits,
invalid/empty queries, count/selection updates, collapse, themes and remounting.
Native accessibility edits are not evidence for OS IME behavior. Full script/bidi,
scrolling and application performance acceptance remain separate release work.

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

The Diff tab now previews an OCaml/JSON patch with per-file collapse, a four-row
preview and native Show more. Application controls expansion switches between
native-managed state and an OCaml reducer that applies typed requests. Word
emphasis is independent of filename-based syntax colors. Append a file streams a
Rust file into the same source; Reset diff starts a new source generation and
restores expansion seeds. File/show-more/line observations update the preview
notice; Enter on a source row demonstrates exact old/new coordinates and payload.
The reducer keeps managed seeds stable when observations arrive.

Build the entire gallery against staged installed public libraries, without
installing into the active opam switch:

```sh
python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/gallery-consumer
```

On macOS, add `--run --gallery-section documents` for the focused native document
driver, or `--run` for the full gallery driver. The driver accepts `--executable`
to rerun an already built independent gallery. A focused pass is not combined
catalog or clean-machine release acceptance.

The initial macOS test covers semantic navigation, button actions, OS typing and
submission, theme/size changes without resetting the editor, independent windows,
repeated page unmount/remount and window shutdown. It is not an OS IME or complete
catalog qualification claim. See the [design contract](../../docs/design/component-gallery.md)
and [catalog inputs](../../docs/catalog/README.md).

## Desktop integration

The gallery runs with identity `com.gpuio.component-studio` and scheme
`gpuio-studio`. One application-owned receiver per service survives page changes;
all windows share the latest link/support observations and one notification
receipt. Notification operations admit one request at a time across all windows.
Terminal actions retire only the matching receipt, and never activate a window.
Document metadata belongs to each exact native window and is observed again on
page entry. Selecting a represented file does not read it; Open and Reveal are
separate actions. Path bytes are not used as UTF-8 display labels.

Support/permission queries do not prompt. Only Allow OS notifications requests
permission; posting success reports OS acceptance, not guaranteed presentation.
Notification services and runtime link registration require a matching macOS app
bundle; direct development launches display `Unavailable`. The Register button
explicitly changes the handler for Studio's declared scheme. It is not called at
startup. `--open-uri=gpuio-studio://preview/startup` demonstrates startup delivery;
`--print-info-plist` emits metadata using the public packaging API without opening
windows. Linux launching requires a session bus because the gallery uses
`App.run_desktop`; full Linux desktop qualification remains deferred.

The focused `--section desktop` check covers direct execution. Packaged macOS
integration has a separate foreground test:

```sh
python3 scripts/test_gallery_desktop_macos.py --artifacts scratch/gallery-desktop
```

It builds an ad-hoc signed disposable bundle from the existing gallery executable,
requests permission for Component Studio through its UI if needed, and tests
actual OS links, notifications and a disposable file-handler fixture. It targets
only fixture applications/notifications, closes its windows and unregisters
fixture bundles afterward. An existing notification denial is not overridden.
This local fixture is not a signed/notarized release or clean-machine acceptance.

The Presentation page's **A useful empty state** card includes
`Presentation.Empty_state` rich helpers alongside the original string helper.
Toggle media/title/description/content/extras independently; compare centered and
leading layouts, narrow wrapping, intrinsic avatar rows, an icon frame and scoped
raster media. A checkbox keeps its Bonsai model as neighboring slots change;
removing its own content slot retires the native control. Run the focused native
check with `python3 scripts/test_gallery.py --section empty`. The root is borderless
by default; reveal its dashed border or apply a solid override. Larger-description
and compact-spacing controls demonstrate proportional line height and an explicit
pixel override without replacing the rich content.


The Presentation page's **Space with intention** card previews
`Presentation.Separator.create`: horizontal/vertical lines, optional labels,
solid/dashed patterns, width and custom colors. The focused geometry/identity
driver is `python3 scripts/test_gallery.py --section separators`; source acceptance
status is tracked in the [separator design](../../docs/design/presentation-separators.md).
`--trace-windows` logs the app's close-request reasons while preserving its normal
Allow decision; it can help distinguish a close request from lost AX access.

The Styling details page includes solid/dashed borders with independent weight
and corner controls. `python3 scripts/test_gallery.py --section borders` exercises
that card's theme, native identity/geometry and retained page state; the separate
`native_border_style` target verifies GPU patterns, state refinements and teardown.
`--background` avoids focus on launch where the platform exposes a usable window;
it does not make keyboard/IME or desktop-pointer checks background-safe.
