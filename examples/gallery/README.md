# Component Studio

## Reading the code

For a first application, start with the [counter](../getting_started/README.md).
For this larger example, follow the files by responsibility:

| File | Responsibility |
| --- | --- |
| [Startup walkthrough](application.md), [main.ml](main.ml) and [application.ml](application.ml) | Entry modes, Eio capabilities, app services, window creation and cleanup. |
| [Reactive composition](component.md) / [component.ml](component.ml) | Bonsai observations and effects; passes resolved values to the shell. |
| [Stateless layout](shell.md) / [shell.ml](shell.ml) | Navigation, page content and window chrome from explicit `Snapshot` and `Actions` inputs. |
| [Page routing and initial demos](pages.md) / [pages.ml](pages.ml) | Active branches plus Presentation, Controls and Text editing; other pages remain separate components. |
| [Shared styling](palette.md) / [palette.ml](palette.ml) and [model/](model/) | Immutable appearance helpers and application value types. |

`View` constructs GPUIO descriptions. `Bonsai` allocates/observes reactive state.
`Effect` describes actions to run later; constructing a view does not run an
application action. The shell owns neither mutable Bonsai state nor Eio work.
File operations are supplied by the application and invoked through scoped page
controllers. Native editor/list state remains in GPUIO's public controllers.
The component pages are demonstrations, not required application boilerplate.

Feedback → **Commands in your workspace** demonstrates a persistent inline
command chooser beside an ordinary native editor. Add steps repeatedly, clear
then cancel with Escape, hide/show while retaining the query, and select the
workspace note through a native edit command. Read
[embedded palette walkthrough](embedded_palette_preview.md) and the
[ownership contract](../../docs/design/palette-embedded.md). The adjacent
[external search walkthrough](external_palette_preview.md) explains dynamic
asynchronous search in a modal, including cancellation and staged result
publication. Its [scope helper](preview_scope.md) explains resource acquisition
and cleanup when a Bonsai branch activates or deactivates.

The [example coverage checklist](../coverage.md#gallery) records companion
coverage for every gallery source part; pending entries remain explicit.

Independent model/support reading: [editor placement visits](model/editor_visit.md)
fence settings persistence; [extension state](model/extension_state.md) serializes
counter commands; [feedback state](model/feedback_state.md) handles notification
identity; [message history](model/message_stream.md) preserves stable row keys;
[follow overlays](model/message_follow.md) decorate that list without owning scroll.
The [section catalog](model/page.md) maps typed navigation identities to titles/keys;
[rating state](model/numeric_state.md), [controlled choice state](model/picker_state.md),
[formatting state](model/selection_state.md) and [retained settings data](model/settings_state.md)
explain how requests are validated against current application values.

Small presentation walkthroughs: [alerts](alert_preview.md), [attachments](attachment_preview.md),
[avatars](avatar_preview.md), [badges](badge_preview.md), [labels](label_preview.md) and
[separators](separator_preview.md). The Styles page also demonstrates
[aspect-ratio layout](aspect_preview.md).

Action and input policy walkthroughs: [live bindings](binding_preview.md),
[button appearance](button_appearance_preview.md), [rich command buttons](button_preview.md),
[standalone radio navigation](checkable_navigation_preview.md),
[command shortcut tooltips](command_tooltip_preview.md) and [native content hints](content_hint_preview.md).

Composition walkthroughs: [description lists](description_preview.md),
[disclosures and retained drafts](disclosure_preview.md), [empty states](empty_preview.md),
[forms](form_preview.md), [group boxes](group_preview.md) and [composed links](link_preview.md).

State and native rendering walkthroughs: [input formats](format_preview.md),
[variable-size card lists](horizontal_list_preview.md), [pagination](pagination_preview.md),
[progress](progress_preview.md), [custom spinners](spinner_preview.md) and
[selectable text shimmer](shimmer_preview.md).

Editing and controlled-input walkthroughs: [sliders](slider_preview.md),
[workflow stages](stepper_preview.md), [numeric drafts](number_preview.md),
[password privacy](password_preview.md), [multiline geometry](textarea_preview.md) and
[prepared edit filters](edit_filter_preview.md).

Native composition walkthroughs: [split actions](split_preview.md),
[retained split panels](split_group_preview.md), [measured card tracks](carousel_track_preview.md),
[shared scrollbar descriptions](scrollbar_preview.md), [independent tab parts](tab_content_preview.md)
and [structural tables](structural_table_preview.md).

Choice and display walkthroughs: [grouped capability pickers](choice_picker_preview.md),
[picker cases](choice_picker_cases.md), [native control indicators](control_appearance_preview.md),
[keyboard labels](keyboard_preview.md), [typed markers](marker_preview.md) and [rich tags](tag_preview.md).

Page composition walkthroughs: [collections](collections_page.md), [feedback](feedback_page.md),
[carousels and journeys](journeys_page.md), [navigation](navigation_page.md),
[overlays](overlays_page.md) and [date/color pickers](pickers_page.md).

Runtime and style walkthroughs: [menu observations](menu_preview.md), [runtime diagnostics](runtime_page.md),
[styling composition](styles_page.md) and [focused-input metadata](window_input_preview.md).

Styles → **A theme from your workspace** loads a palette for the current window.
Read the [theme-loading walkthrough](theme_preview.md), then the independent
[appearance model](model/appearance.md), [selection identity](model/theme_selection.md),
[profile decoder](model/theme_profile.md) and [Eio file adapter/test](files/theme_file.md).
Choose [the Aurora sample](themes/aurora.sexp), edit its hex colors in your editor,
save, then use **Reload file**. Type into **Theme preview draft** before reloading
to check that the same editor keeps its text. A malformed file leaves the last
working palette in place. Light/Dark and Follow system leave the custom profile;
a pending file result cannot override that newer choice. Each window has its own
selection. Leaving Styles cancels its pending file work while retaining the
already applied palette.

The example format is a versioned S-expression with a name, Light/Dark appearance
and ten concrete hex colors, bounded to 16 KiB. It changes application colors and
the document light/dark mode; logical size remains controlled by the size button.
Reload is explicit, with no directory watcher. See the
[contract](../../docs/design/gallery-theme-files.md) and
[local validation](../../docs/evidence/gallery-theme-files-och41.md).

The Runtime page includes **Selection without the clipboard**. Drag across its
sample lines and press Primary+Shift+U to inspect the text; Primary+Shift+E ends
the drag while preserving the range. Primary+Shift+Y checks presence and
Primary+Shift+K clears selection; shortcuts avoid clicking away from the range.
These read-only text/document queries do not inspect editor
values or access the clipboard. The default result budget is 65,536 UTF-8 bytes;
applications can choose 0..262,144 bytes. Oversize returns `Limit_exceeded` without
partial text. The [physical macOS walkthrough](../../docs/evidence/window-selection-och41.md#physical-public-gallery-walkthrough--2026-10-05)
passes exact cross-node Unicode reads, end/clear, independent windows and native
selection retirement on page unmount. Shortcuts resolve within the card's focused
registry; focus a card button when querying an empty selection.

A native public-API gallery under implementation for OCH-41. The first sections
cover presentation, selection/actions, native text editing, numeric/range inputs,
verification codes, rating, dates/colors, overlays, navigation/layout and commands/
feedback, carousel/sidebar/history, managed collections, documents and runtime/
window previews, canvas, images/icons, charts, plus native motion sequences, shared clocks, responsive layouts, native extensions, input/transfers, desktop services, styling details, general input observations and subtree highlighting. Additional v1
families and the complete coverage ledger are still being integrated; this is
not the completed milestone 07 release.

Collections → **Searchable list** demonstrates the public managed selection and
Eio search APIs with 1,000 entries, sections, disabled options and a 16-row budget.
Select entries, filter them out and return: selection stays with the original
collection. **Fetch remote**, **No matches**, **Try recovery** and **Retry search**
exercise asynchronous results and failure recovery without a network service.
Change orientation, inspect a row, update its detail or start a new collection.
Arrows move the cursor, Space on the list toggles selection, Enter confirms and
Command-Enter previews; the query retains its own editing/composition behavior.
Use `scripts/test_gallery.py --section selectable-lists` for the physical macOS
walkthrough when desktop automation is available. See [search evidence](../../docs/evidence/selectable-list-search-och41.md)
for the current local coverage and remaining physical qualification.

Navigation includes **A workspace that adapts to you**, built with public
`View.split_group`: edit a retained draft, reorder/hide/insert/remove panels,
switch axes, apply size limits, request a resize, reset sizes and enable custom
divider grips. Native dragging, keyboard resizing and completed size observations
stay independent of the draft's lifetime. Local TestPlatform and installed-package
build checks pass; the [physical walkthrough](../../docs/evidence/split-group-bridge-och41.md)
remains part of macOS release qualification.

Feedback's **A quiet confirmation** now has **Placement** and **Reserve window
margins** controls. Save a notification, then move the same live toast among the
eight edge/corner anchors or reserve asymmetric window insets. Placement changes
preserve its native timeout and child lifetime. **Layer notification cards** now shows three persistent, independently dismissible cards with different
heights. Hover or Tab to the named Notifications Group to expand; with the Group
focused, Page Up/Down and Home/End scroll taller stacks. **Show three sample
notifications** restores dismissed examples. **Animate notifications** enables native
entry/exit and spring reflow independently of layering. Turning it on preserves
already-present cards; new saves or restored samples show entry, and dismissal
retires input before the exit finishes. Reduced motion settles immediately.

Selection & actions includes **Two actions, one control**, using the public
`View.split_button` API. Cycle split/action-only/menu-only modes, disable the
primary independently, mark it loading, or disable the pair. Menu commands share
the action counter but retain native navigation and separate focus. The primary
has individual tooltip help. Shared hover/menu-held painting happens natively;
the example does not subscribe to menu visibility to drive its colors.
`--section buttons` includes the split keyboard/accessibility walkthrough. It is
authored but unrun; physical hover pixels, corner/seam quality and full native
acceptance remain open. Build both runtimes from this checkout: the unpublished
epoch3 bridge includes mandatory split operation70 and the Link loading payload.

**Actions, in context** includes a separate **Load link preview** toggle. Its
composed Link stays focusable while busy, suppresses activation, and keeps the
same native owner through loading/disabled/recovery cycles. The application
chooses its loading label; the library does not manufacture spinner artwork.
The appearance card also uses `View.with_hover` on each action and displays the
current native hover observation. This requires paired unpublished epoch3 Op71
and Event68. The extended desktop walkthrough is authored and unrun.

The Numeric inputs page includes independent rating colors, 1/5/10-star maxima,
16/24/36-pixel star boxes, disabled/read-only policy and an optional step-down
click reducer. The default reducer still clears an already selected star.
`scripts/test_gallery.py --section rating` is the dedicated native walkthrough;
its new geometry/input checks and the new native GPU color fixture await actual
desktop execution. Compilation does not establish their acceptance.

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Launch with `--custom-chrome` to exercise the public `View.title_bar` composition
and `Window.Chrome.Custom`. Drag empty title-bar space or double-click it; child
buttons must remain independently usable. Toggle fullscreen to check the macOS
traffic-light reservation. Native macOS/server controls remain OS-owned; Linux
client decorations use `View.window_controls` for supported Minimize/Maximize and
Close, through the same application close-decision path. Add `--trace-windows` to
log those decisions, or `--background` for layout-only work. The Runtime page's
**Minimize this window** command is separate; restore the window through the OS.
Linux custom chrome requests client decorations and automatically supplies the
frame shadow, border and resize handles. Tiled sides and fullscreen remove their
visual insets; popup and overlay fitting stays inside the frame. The frame border
follows the gallery theme. `Window_frame.create` configures shadow margin and
resize half-band through `App.open_window ?frame`; native macOS/server frames
remain OS-owned. Explicit regions also remain available through
`View.with_window_region`. This example's build and TestPlatform checks do not
establish physical macOS/Linux chrome acceptance.

The Runtime page's **Which input owns focus?** card uses
`App.Window.focused_input`. Focus its draft, masked or read-only field, then press
Command+Shift+I (macOS) or Ctrl+Shift+I (Linux). A keyboard command preserves the
field's focus while the asynchronous query matches its controller identity and
reports the fixed field label and kind. No text or protected value is read by this
query. The [physical macOS walkthrough](../../docs/evidence/window-input-query-och41.md#physical-public-gallery-query--2026-10-05)
passes ordinary/masked/read-only owners, retained native editing, independent
windows and fresh identity after remount.

Use the sidebar to switch previews, the light/dark button for explicit appearance,
**Follow system** for the palette matching native window appearance, and the size
button for compact/comfortable/large logical sizing. Startup uses explicit Dark;
choosing light/dark leaves Follow system. Native appearance changes do not override
an explicit palette. See [the appearance contract](../../docs/design/window-appearance.md)
and [physical macOS validation](../../docs/evidence/window-appearance-och41.md#physical-macos-appearance--2026-10-05)
for two-window switching, retained native editing and preference restoration. This sizing is not OS
DPI emulation. Each new window has independent navigation, appearance and editor
state. Leaving a page unmounts its native editor leases; appearance/size changes
on the current page preserve native text and selection. Preview model state is
local to its Bonsai branch and survives page changes. Transient modal, chooser and
in-app toast state is cleared on departure. Application-owned OS notification/link state survives page and window changes. At most four windows are opened by the gallery.

The **Settings** page composes the public Settings model/panel/field APIs with
application-owned values and native editor controllers. Explore two settings pages,
48 Advanced groups, typed switches/checkboxes, text, independent numeric drafts,
three regions, a 250-option model dropdown, a policy-locked custom control and
four group variants. Narrow/wide changes retain the editor; filtering/page visits
restore stored text and numeric drafts with fresh native editing sessions.
**Export settings…** asks for a destination and writes a committed-value preview
snapshot through Eio; **Try failed export** deliberately injects a task failure
without dropping edits. Export is an example format, not an application settings
service. The focused driver is `python3 scripts/test_gallery.py --section settings`.
The repository and a fresh installed consumer pass scoped interaction checks,
including consumer long-choice keyboard selection and Unicode paste.
Whole-page resets now also restore saved values for filtered-out editors; retired
placements cannot overwrite the new seed. Group resets, export validation and
actual native Save/Eio readback pass focused checks. Complete field edge coverage
and full native acceptance remain open;
see the [Settings contract](../../docs/design/settings-composition.md).

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
See the [display contract](../../docs/design/keyboard-labels.md).

The **Shortcuts in context** card uses `Gpuio_bonsai.Command_binding.component`
to show live application and native Copy bindings. Switch between focused,
specific-editor, local-declaration and native-context queries; enable/register
commands independently of showing their bindings. Native strokes use
`Presentation.Kbd.of_native_stroke`, including ordered sequence display.
Query changes preserve the editor placement. Run
`python3 scripts/test_gallery.py --section binding-observations`; the section
also runs in `core` and `all`. See the [native observation contract and remaining
acceptance](../../docs/design/command-binding-observations.md).

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

The Presentation page's **Custom spinners** card demonstrates `View.spinner`
with a scoped SVG arrow, built-in strokes and a failed-icon fallback. Toggle
animation, cycle length and easing; the icon inherits the current theme color.
The native owner retains timing and asset observations without per-frame OCaml
updates. See the [spinner contract](../../docs/design/custom-spinner.md) for
lifecycle rules and the still-open native GPU/accessibility acceptance checks.
The focused driver is `python3 scripts/test_gallery.py --section spinners`, also
included in `all`. It is authored but awaits native execution; screenshots and
keyboard assertions must pass before claiming gallery acceptance.

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
Collections starts with 1,000 loaded records and bounded active rows/cells. The
message preview adds up to 32 earlier and 32 later messages, grows or resets the
latest response, and reports native tail-following state. **First entry** pauses
following; **Jump to latest** resumes it. The native animated **Follow latest**
overlay and optional bottom fade appear only while away from the live edge.
**Show follow button**, **Fade transcript edge** and **Animate follow controls**
configure that presentation independently. The list retains its size and scroll
owner; hidden buttons leave focus/accessibility immediately and settle outside
the clipped viewport. See the [public-API recipe](../../docs/design/message-follow-presentation.md).
Growth updates only the latest value,
preserving the order snapshot. These controls are manual so the example starts
no background producer. Its named native Log uses Live.Off; it does not announce
every fragment or expose unmounted history. The **Result table** has 1,000 rows and
controls for **Row headers**, **Wrap table navigation** and **Select only entry
header**. Restricting header selection leaves all cells and sorting usable; a
selected ineligible column clears. These options retain the native table and
surviving cell models. With headers hidden, clicking the selected cell again
selects its row. See the [behavior contract](../../docs/design/table-behavior.md).
The table also has **Striped rows**, **Table part colors** and **Compact cell
padding** controls. Part colors follow the gallery palette; compact padding applies
a shared inset with zero padding in ENTRY. Selection and native viewport state
survive these updates. See [table presentation](../../docs/design/table-appearance.md).

The table also displays its observed visible column IDs, pinned status and
partial visibility. Horizontal scrolling updates the readout only when those
bands change; this does not change mounted-cell budgets. The observation also
works for empty-data headers. See [column viewport](../../docs/design/table-column-viewport.md).

Collections → **Structural table** demonstrates fully mounted `Table_view`
composition: grouped headings, column spans, row headers, footer and caption,
with native Open buttons and keyed row reversal. It has no managed-row controller
or selection model. The explicit table label and derived counts/indices reach
native accessibility; embedded controls keep their own actions. See
[structural tables](../../docs/design/structural-tables.md).
Internal preview panels
retain native state; leaving the whole page unmounts them.
The Documents page also has a **Compact preview**. **Expand preview** and
**Collapse preview** change its public Markdown Flow line budget without
republishing the source. The adjacent status comes from `on_preview` and
separates rich overflow, pending content, collapsed bodies and source fallback.
Resize the window and append/reset the document to exercise native observations.
The existing fixed-height viewport remains a separate example.

The Documents preview includes **Copy selection as Markdown**. Select rich body
text and compare Copy with the option off/on: plain rendered text versus Markdown
reconstruction (select-all retains the original Markdown). Copy source/code/table
buttons keep their existing semantics. Changing the option preserves the native
selection and source revision; code/source fallback already copies displayed
source. This public example builds locally; physical clipboard/keyboard validation
of the toggle remains a separate release check.

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
with horizontal/reversed value axes, disabled and data-update controls. Read the
adjacent [charts walkthrough](charts_page.md) for the independent application-data,
Bonsai, GPUIO and scoped-runtime parts. The pure [axis/grid preset guide](chart_axes.md)
explains physical-fraction ticks and styling; Cartesian axes transpose with
orientation, while candlestick axes remain vertical. These controls change
presentation without publishing new source data and are still under qualification. Each page visit acquires one
scoped registration; leaving releases it. Choices survive a revisit while source
phase and selection start fresh. Read the pure [mark preset guide](chart_marks.md) for source-ID highlights,
path/bar styling and explicit aggregate sampling in Cartesian/radar modes. Pie,
Candlestick, Ordinal colors and flow modes hide these controls; integration remains
under qualification.
The original-data table includes node records
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
per-gesture source/target transitions (not every move), incoming origin and the
hex-encoded path bytes of accepted file offers. Enable it only when those paths
may be logged. The native tests use it to verify gesture identities, exactly-once
delivery and unchanged file paths from a separate AppKit source process.
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

The **Feedback** page includes circular progress with an editable center, rounded
linear progress and empty/tiny/quarter/nearly-complete/full/indeterminate targets.
Toggle value interpolation or inert presentation to examine native motion and
retained center state. It uses `View.progress_circle` and optional
`View.progress ~transition`; updates travel as semantic targets, with animation
owned natively. The new preview's actual GPU, accessibility and keyboard acceptance
remain pending; a successful gallery build is not those runtime checks.

The focused progress driver is `python3 scripts/test_gallery.py --section progress`
(or `--executable` for an installed gallery). Its native assertions are authored
but not yet executed; offline ring-sampling checks do not establish GUI acceptance.

The Controls page now includes command-based formatting and alignment toolbars,
a mixed master checkbox, disabled-request handling and orientation changes.
**Connected formatting buttons** switches between shared seams and separate
buttons. **Single item groups** shows one retained button in each toolbar, with
all outer corners and borders; restoring the group retains its selection model.
`python3 scripts/test_gallery.py --section selection` is the focused desktop
walkthrough. Its native assertions are authored but unrun; local model, paired
codec and AccessKit metadata tests and the gallery build pass. See the
[selection review](../../docs/catalog/selection-review.md) for explicit remaining
family gaps and platform limits.

The Controls page also includes a public `Control_appearance` preview: custom
or default checkbox/switch/radio artwork, 18/32-pixel size, label order, selected
and mixed colors, disabled/inert policy and an individually disabled radio
option. Theme changes preserve keyed controls and selected values. Native desktop
validation of this new preview remains pending; headless library tests do not
establish GPU, keyboard or AX acceptance.

Run `python3 scripts/test_gallery.py --section control-appearance` for the new
appearance walkthrough, or add `--executable` to use an independently installed
gallery. It is included in `all`. The driver is authored but unrun; its offline
pixel-sampling checks and the independently installed consumer build pass.

The same preview now has **Rich control labels**. It switches the existing keyed
checkbox, switch and radio group between strings and multiline passive View
labels. Radio overrides are keyed by choice ID; Fast demonstrates ordinary string
fallback. The rich constructors require checked names/content and preserve one
native action owner. Headless routing, disabled fallback styling, animation and
cleanup tests pass. The expanded appearance driver includes real caption clicks
and AX ownership checks, but those desktop scenarios remain unrun.

“Compose a choice your way” demonstrates `View.radio` and `radio_with_label`,
application-owned selection and `Accessibility.Role.Radio_group Vertical`.
Each choice has its own native focus handle. The controls reverse signed Tab
order, skip Fast during Tab traversal while preserving pointer selection, disable
choices and switch rich/plain labels. Clearing the navigation toggles restores
default order without replacing the keyed controls. Use the managed `radio_group`
when you want one group Tab stop and Arrow navigation. Production-View headless
checks exercise click, Space/Enter press/release, Tab/Shift-Tab, checked no-op,
identity and cleanup; real macOS and installed-consumer runtime acceptance remain
pending. See the [navigation contract](../../docs/design/checkable-navigation.md).

The Controls card **Actions, in context** demonstrates default, primary,
secondary, danger, info, success, warning, ghost, link, text and custom palettes.
It uses ordinary public styles for outline, compact/large sizing, rounded corners,
selected appearance, native hover/pressed/focus and disabled states. Link uses
`View.link`; selected action appearance does not invent toggled semantics. Hover
or focus reveals plain help; Primary has a rich tooltip with preferred right-side
placement. Activating an example only increments a local preview counter.
`--section buttons` includes its desktop action/tooltip/geometry/identity driver,
which is authored but unrun. See the [button review](../../docs/catalog/button-review.md).

**Menus that report their state** demonstrates `menu_button ~on_open_change`
with native open/close observations, optional subscription, disabled menus and
preferred root placement. Submenu navigation and command invocation stay in Rust;
the counter and visibility label are ordinary Bonsai state. The application does
not control navigation through the callback. This requires paired protocol epoch3.
`--section buttons` includes its authored, unrun desktop walkthrough. Native
TestPlatform checks cover lifecycle, focus, command ordering and one placement
configuration; full desktop/placement qualification remains open. See the
[menu contract](../../docs/design/menu-observation.md).

The selection card's **Bold option busy** and **Enable Italic option** controls
demonstrate per-child loading and disabled policy while other options remain
usable. Passive rich labels retain the command owner; busy Bold adds a spinner.
Bulk selection changes only available options and leaves locked values intact.
The master checkbox reflects all selections, so it can remain mixed until an
unavailable option is enabled again. Group disable takes precedence for actions.

**Hints that follow the action** combines a command button, managed tooltip and
live command-binding observer. Focus or hover **Run hinted action** to see its
assigned shortcut. Change the chord, remove its registration, disable the action
or switch display platforms. Removing the shortcut keeps the ordinary button
action available. Hints describe local registry declarations; OS-reserved chords
and focused-context conflicts are not a shortcut-delivery guarantee.


The Pickers page also contains “A workspace, tailored to you”: a public
`Gpuio_eio.Choice_picker` example with grouped multiple selection, a retained native
search field, rich option rows and a footer clear button. Its Bonsai reducer applies
selection requests to current state. The example builds locally; its new desktop
walkthrough and visual/input/accessibility qualification remain pending. See
[the picker contract](../../docs/design/choice-picker.md) for the experimental API
and current coverage limits.

Three additional public picker cards now sit on that page:

- **A destination you control** uses single selection and application-controlled
  opening. Turning off changes disables the chooser and rejects queued requests;
  native visibility observations remain separate from the requested open state.
- **Your next workspace** searches 4,096 catalog entries. The catalog contains
  immutable option data; it does not allocate 4,096 OCaml View nodes.
- **A fresh start** supplies custom empty content and an interactive footer that
  creates the first workspace. Reset returns it to the empty catalog.

`python3 scripts/test_gallery.py --section choice-pickers --background` is the
new desktop walkthrough. It checks grouped selections, controlled policy,
large-catalog search/draft retention, and empty/create/reset behavior. It uses
AXValue to set search text and does not claim physical typing or IME coverage.
The walkthrough is authored and Python-compiled, **not yet run**. Model tests,
full OCaml checks and the gallery build pass; visual review remains pending.

The Text inputs password card now uses the public `View.input_frame` helper:
leading content, focus-preserving application-controlled reveal, undoable native
clear and a loading/busy indicator that leaves typing available. Read-only,
disabled and edit-menu controls remain independent. Use synthetic sample text.
The gallery is under validation; TestPlatform evidence does not replace physical
macOS input/IME, external accessibility or visual review.

The Text inputs page includes a semantic-hint preview. Focus the sample contact
field, change its email/URL/IMEI hint and use **Check native hint** to inspect the
asynchronous result without moving focus. The label shows the hint at query time,
so changing config does not relabel an older observation. An exposed native
property is not a promise of autofill. The password example uses explicit password
privacy and its corresponding content hint.

The Editors page's **Room to write** card keeps one native text area while
switching wrapping, continuation indentation, whitespace indicators and cursor
margins. Use arrow navigation in the long draft to explore margins; explicit
selection retains native minimal-reveal behavior. Layout switches preserve edits,
composition and undo history. The new preview has local automated layout/state
coverage; physical whitespace appearance and desktop input acceptance remain
open. See [text-area layout](../../docs/design/textarea-layout.md).

The Editors workspace form also exposes **Escape clears editable drafts** for its
title/body fields. Enable it, edit and press Escape, then undo. Compare with the
read-only toggle; the first Escape during composition only ends the native mark.
This is opt-in and leaves normal empty-field propagation available.

**Room to write** also has **Back to top**, **Jump to end**, and **Inspect view**.
The first two request native scrolling while preserving selection and composition.
Inspection reads the last completed layout, including its buffer-line overscan;
it does not poll or claim that a preceding scroll request has already painted.
See [editor viewport commands](../../docs/design/editor-viewport.md).


**Room to write** now includes **Find in notes** and **Find and replace** using
`Gpuio_eio.Search_bar`. Try repeated words, case matching, previous/next, and
undoing a replacement in the document. Primary-F reopens the query;
Primary-Shift-F shows replacement; F3/Shift-F3 navigate, query Enter/Shift-Enter
navigate, and Escape closes. Document Enter still inserts a newline. Closing
restores document focus when eligible. Composition, delayed replies, replacement
stamps and local teardown have automated coverage; physical desktop focus/IME,
accessibility and visual qualification are still required. See the
[search contract](../../docs/design/textarea-search.md).

**Numbers with clear intent** starts with draft `1e-` and committed quantity `12`.
Restore the committed value, change between side/stacked/keyboard-only controls,
then edit with quarter steps in the range 0–100. Commit/restore and undo/redo show
that editing history and committed value are separate. Allow-empty and disabled
switches update the retained field. The page's read-only switch applies too.
Toggle the complete frame and custom step symbols while editing: the Qty prefix,
units suffix, themed frame focus and button hover/pressed states surround the
same editor. Qty has an independent action; whole-control disable also disables
it, while read-only leaves it available. See the
[numeric presentation contract](../../docs/design/number-presentation.md).
“Choose step size in OCaml” enables the public application-step mode: quarter
steps below 10, single steps below 50, then steps of five. The request handler
uses the native draft snapshot, selects a proposal and resolves its original
editor/revision. Hold a step button to repeat; release stops further requests.
The native tests cover delayed replies and cancellation; the gallery walkthrough
still needs physical desktop acceptance. See the
[request contract](../../docs/design/number-step-requests.md).
The expanded card builds and existing gallery tests pass; its desktop walkthrough
and visual acceptance remain pending.

**A six-digit verification code** has live grouped/ungrouped and larger-cell
controls alongside masking and read-only policy. Type a partial code, then change
grouping or size: the same native draft, selection and undo history survive.
Cells use the current palette's surface, border, accent and foreground colors.
Automated TestPlatform checks also cover changes during composition; actual
macOS IME and visual qualification remain open. See the
[OTP presentation contract](../../docs/design/otp-presentation.md).

With the OTP field focused in an active window, its caret now blinks natively.
Typing restarts the visible phase. Composition, read-only and reduced-motion
policy use a steady caret; hidden, clipped or inactive fields retain no blinking
task. Fake-clock/native paint checks pass; physical macOS timing and input-method
qualification remain open. See [caret lifetime](../../docs/design/otp-caret.md).

The Navigation page includes **From an idea to a launch**, built with
`Gpuio.Stepper`. Click stages or use Tab/Enter/Space; Previous/Next skip the
optionally disabled Review stage. Change orientation, centered labels and status
symbols while keeping the same notes draft. Whole-navigation disable and stage
requests share one model. “Completed” denotes stages before current, not validated
or submitted work. Exact public-view/native TestPlatform checks pass; the expanded
card still needs physical visual/keyboard/VoiceOver acceptance. See the
[workflow contract](../../docs/design/workflow-stepper.md).


On **Dates & colors**, the calendar's **Event badges** switch changes passive day
and header artwork. **Update events** edits the count on September 14 without
replacing the native calendar or changing selection. The same checked content is
used in the appointment picker. Unspecified dates keep native labels. The authored
`--section pickers` walkthrough includes retained AX identity/description and
selection checks; the new content checks have not yet run on a physical desktop.


On **Overlays & help**, **Tinted backdrop** switches modal artwork between an
accent tint and transparency. The dialog's **Change backdrop** button updates it
while open; transparent backdrops still trap focus and block background input.
The authored overlays walkthrough checks retained dialog identity and `AXModal`;
these new physical checks have not yet run.

The Overlays page includes **Animate opening**, using `Overlay.Motion.Enter`
for dialogs, confirmation and sheets. Disable it for immediate presentation.
Reduced motion settles automatically. **Change backdrop** inside the dialog
updates paint without restarting entry or replacing controls. Animated surfaces
hold focus on the modal scope until settled unless the user focuses a visible
child; closing always releases content immediately. See the
[entry contract](../../docs/design/overlay-motion.md).

The same **Animate opening** toggle now controls entry/switch presentation on
three adjacent help triggers: **Focus for a tip**, **Streaming help**, and
**Keyboard help**. Move the pointer or keyboard focus between them to compare
managed replacement and immediate mode. The retained hover-card example keeps
its existing presentation. [Tooltip contract](../../docs/design/tooltip-motion.md).

The Overlays page also includes **Open placement preview**. Its controls switch
between trigger and window-point placement, choose each corner, and change the
viewport margin while the popup stays open. Resize the window, tab between the
controls, close with Escape or the close button, and check return to the trigger.
This is a public Core/Bonsai composition; no private native API is required.
Physical visual/focus qualification remains separate from the TestPlatform tests.

Inside **Open drawer**, **Reserve space for app chrome** changes sheet insets
while retaining the open content. **Move drawer to next edge** cycles right,
bottom, left and top. Resize and toggle the reserved band during entry and after
settling; Escape/close must restore focus to the opener. The backdrop still
blocks the reserved band. Native TestPlatform checks cover these layout rules;
physical desktop qualification remains required.

The Pickers page subscribes independently to the inline calendar and popup date
picker's logical viewport. Navigate across months, switch month/year selection,
and change **1/2/3/12 months**: the status describes the exact grid-date span and
sparse event badges/headings follow it. **Update events** preserves the calendar
owner and draft. App content loading belongs in a scoped Eio task, guarded against
outdated observations; this deterministic gallery uses local sample data. See
[viewport lifecycle](../../docs/design/calendar-viewport.md). Physical navigation
and pending-Apply interaction remain part of desktop qualification.

On Journeys, **Branch selection: navigate/expand/toggle** cycles the Projects
branch's activation policy without rebuilding the sidebar. Select Projects twice:
navigate leaves expansion unchanged, expand keeps it open, and toggle changes it
on each selection. The separate caret changes expansion without navigation.
Compact icon mode keeps descendants hidden and preserves the chosen expansion
state for reopening. These controls use `Sidebar.Item.Activation` and the standard
current-model reducer; physical desktop qualification of this addition is open.

On Journeys, **Style sidebar labels** adds/removes per-item rounding and a featured
Projects text style while preserving the same native links. The adjacent count
suffix has its own appearance and disappears in compact mode. Compare expanded,
icon and offcanvas modes, selection and the independent caret. See the
[styling contract](../../docs/design/sidebar-styling.md); this walkthrough still
needs physical desktop qualification.


Navigation's **Independent tab controls** preview uses `View.tab_bar_with_content`.
Close a selected or inactive tab, reverse or restore the current set, and toggle
long-name truncation. Close is its own focusable button and does not also activate
the surrounding tab. Configured names stay complete in accessibility. This preview
has a bounded native horizontal viewport. **Select last tab** updates selection
without moving that viewport; **Reveal last tab** requests scrolling without
changing selection. Arrow/Home/End navigation and focus on a child control reveal
their native targets. Wheel scrolling stays put across unrelated model updates.
This preview is build-tested; its new physical desktop walkthrough remains unrun.

The preview now uses `View.tab_bar_frame`: **Workspace** and the **+** restore
button remain outside scrolling, while a separate **Restore** button lives at
the scroller's trailing edge. Prefix/suffix/trailing views keep their ordinary
focus and actions. Layout and native focus reveal pass TestPlatform checks, and
a fresh installed-gallery build passes; the physical frame walkthrough remains unrun.


The fixed **All tabs** caret opens the current full-name menu, with selection
checks and disabled choices. Keyboard navigation/typeahead and selection happen
in the native popup. Choosing a row changes selection without implicitly scrolling
the tab viewport; **Reveal last tab** remains a separate action. The menu's
TestPlatform behavior is verified; its physical keyboard/VoiceOver walkthrough
remains open. **Show tab menu icons** toggles scoped decorative SVGs beside the
full names. Reordering and closing tabs also updates the menu's keyed icon rows.


The Navigation page's closable workspace tabs expose **Animate tab selection**.
This opts into `Tab_bar.Motion.default`: an interrupted native Pill indicator
spring and inherited selected-foreground fade. Reverse, select/reveal, resize,
close/restore and menu controls remain independent. Disabling it returns to the
static selected presentation. Reduced motion settles immediately.

Collections now shares **Scroll your way** controls across its message list,
outline, result table and new **Scrollbars** tab. Choose Always visible, On hover
or While scrolling; cycle Both/Horizontal/Vertical; toggle gradient thumb states,
finite motion or the entire custom description. These controls use only public
`Scrollbar` values and `View.with_scrollbar`. Turning custom presentation off
restores the viewport's default (ordinary containers have no default painted bar).
The two-axis preview adds up to sixty keyed rows and resets to twenty-four without
replacing its native scroll owner. Existing message history/growth controls remain
available with the same presentation.

Collections → **Use system preference** reads the current macOS scrollbar style
and applies it to the collection previews: overlay becomes “While scrolling”,
legacy becomes “Always visible”. Refresh explicitly after changing system settings.
Linux reports that the preference is unavailable and keeps the existing choice.
This action does not change OS settings, subscribe to changes or install a timer.
Leaving the page invalidates an outstanding reply; each window owns its choice.
See [the snapshot contract](../../docs/design/scrollbar-preference.md).

Manual scrollbar walkthrough (physical macOS qualification still pending):

1. Open Collections → Scrollbars. Drag each thumb; verify the two tracks leave a
   shared corner and reach both ends. Tab to a range and use arrows, Page Up/Down,
   Home/End. Escape during a drag should leave the previously focused control alone.
2. Add rows while partway down; toggle gradients and motion without losing position.
   In On hover/While scrolling, let the finite idle interval finish, then reveal
   the bars again. Reduced motion should settle transitions immediately.
3. Cycle the axis filter: it hides the excluded bar without preventing ordinary
   content scrolling. Disable custom presentation, then restore it.
4. Switch to Result table. Its vertical track starts below headers, and the
   horizontal track starts after the pinned entry column. Preserve a selected
   cell while using range keys or cancelling a drag. Verify appearance/scale
   changes, visibility, tab switching and teardown on the physical desktop.

See [table and gallery evidence](../../docs/evidence/scrollbar-table-och41.md).

### Horizontal managed cards

Collections → Horizontal cards uses `Virtual_list.Config.horizontal` and
`Gpuio_bonsai.Virtual_list.component_with_config` through public APIs. It starts
with 10,000 unequal-width cards and at most 16 active row computations. Use First,
Middle and Follow latest; scroll horizontally while reading, Grow visible card,
Reverse order, Prepend and Append. The visible logical key stays anchored when it
survives a data/configuration change. React to a visible card, then switch axes:
a row that remains active keeps its local reaction count. Evicted transient row
state follows the documented reset policy. Shared scrollbar controls apply here.

Changing axes replaces native measurements and capture, preserving collection
identity. The preview waits for a fresh geometry observation before presenting a
current viewport. The scrollbar axis selector controls presentation, so choosing
only the perpendicular bar can hide the active-axis bar without disabling scroll.
The main-axis wheel gesture is used directly; vertical gestures are not remapped
to horizontal scrolling. Physical macOS pointer/keyboard/VoiceOver qualification
for this preview remains unrun; compilation/TestPlatform are recorded separately.

Collections → **Result table** → **Rich table headers** mounts ordinary Views in
native leaf/group header cells. **Inspect** is a header button that changes the
notice independently of sort/selection; **RESULT DETAILS** spans the two unpinned
columns. Resize or reorder those columns, change table padding/theme, then toggle
rich headers off and on. Body rows remain virtualized and keep their separate
active-cell budget. Header computations belong to the OCaml caller; Rust renders
submitted content without callbacks into OCaml. See the
[header contract](../../docs/design/table-renderer-slots.md) and
[local evidence](../../docs/evidence/table-header-rendering-och41.md).

The Documents page also offers **HTML**: native reader headings, paragraphs,
emphasis, links, lists, quotes, registered images, preformatted code and a table.
**Append HTML** and **Reset HTML** exercise the same scoped revisioned source.
The external image URL deliberately remains an alternative-text placeholder;
only the registered prism asset resolves. Selection copies plain text, while
Copy source preserves HTML. This is a reader, without browser scripting or general CSS layout.

The Documents page's **Refined reader styling** switch applies to Markdown, HTML
and Images. It demonstrates the public internal palette, six heading sizes,
paragraph spacing, inline-code highlights and code/table/header/cell refinements.
Switching it preserves the source and selection while native row heights adapt.
See [the style contract](../../docs/design/document-styling.md).

Document link notices show input source, mouse button and modifiers through
`Document.Navigation.Link { url; activation }`. Try an ordinary click, a modified
click, middle/right click or Tab then Enter. Routing remains in the application;
the gallery records the request without opening a browser. Keyboard metadata also
represents the native synthetic accessibility route. See
[the link contract](../../docs/design/document-link-activation.md).

The Markdown page exposes **YAML frontmatter**, **Metadata descriptions** and
**MDX syntax**. Enable YAML and switch descriptions to compare native metadata
rows with code. **Try unsupported YAML** loads quoted/sequence syntax that stays
visible as code; **Reset document** restores the mapping example. Select metadata
labels/values, resize for wrapping and append findings to exercise retained text.
The MDX wrapper keeps child content and literal expressions without evaluation.
Copy source retains the original spelling. See
[parser options](../../docs/design/document-markdown-options.md) and
[frontmatter descriptions](../../docs/design/document-frontmatter.md).

The Documents page's Markdown and HTML examples expose **Custom block actions**,
**Enable custom actions** and **Native Copy buttons**. Inspect a snippet or summarize a
table to see an OCaml notice from the native snapshot (including revision and
input source). Copy stays local to the native clipboard. Append or change styling
with the controls enabled to exercise retained parsing and wrapped action rows.
See [document actions](../../docs/design/document-actions.md).

**Native document profile** on Markdown/HTML demonstrates a separately packaged
Rust reader extension through a typed OCaml instance. Toggle **Amber code
highlights** to replace its properties; code/table buttons report typed actions
and installed source revisions in the notice. Markdown also has review badge/card
syntax handled by the package's inline/block plugins. Clearing the switch restores
the ordinary reader/declarative action slots. See the
[profile package](../document_profile_package/README.md) for the schema and API.

The gallery requires both `example.counter` and `example.document` in its compiled
backend. `main.exe --check-catalogs` validates those schemas in a fresh process and
exits without opening a window. The independent-consumer script now performs this
check and the copied profile expect tests, even without `--run`. That establishes
linkage/schema/OCaml behavior; it does not substitute for native focus/clipboard/
VoiceOver or GPU validation.

Remaining physical profile walkthrough: enable the profile in both reader formats,
activate code/table controls with mouse, Enter/Space and VoiceOver; inspect the
revisioned notice; exercise the Markdown review controls; change highlight palette,
append/reset source, toggle the profile off/on, and leave/revisit the page. Confirm
clipped/offscreen controls cannot activate and that focus restores appropriately.
This walkthrough is documented, not yet recorded as passed.

Launch with `--document-defaults` to install application-wide Markdown selection
copy and wider paragraph spacing through `App.run_desktop`. Documents →
**Application reader defaults** compares **Inherit**, **Built-in** and **Compact
override** against the same registered source. The comparison suppresses any
inherited profile so it demonstrates settings independently. Normal startup uses
empty application defaults. See the [defaults contract](../../docs/design/document-defaults.md)
for typed per-field resets, mode restrictions, shared callback context and immutable
application lifetime. Physical selection/copy/layout verification remains pending.

Runtime → **Minimize this window** exercises `Window.Command.Minimize`. Restore
from the Dock/window manager, then verify that drafts and the window's tasks are
retained. This is a manual physical qualification step, not an assertion that the
command's returned snapshot reports a minimized state. Custom title-bar/resize
regions remain open in [the window review](../../docs/catalog/window-review.md).


### Inspect an editor selection

In **Room to write**, select a caret, word or multiline range and choose
**Inspect selection bounds**. The inspector reads a native text snapshot and
queries that exact source revision, showing logical window-content coordinates.
It preserves the draft, selection and focus. Bounds are a snapshot of completed
layout, include endpoint carets, and are not clipped or a visibility receipt.
A source edit between the two requests asks you to inspect again; an endpoint
outside the retained layout yields an unavailable result. Try wrapping, scrolling,
Unicode, and multiple gallery windows. Leaving the page suppresses a late reply.
This walkthrough still needs physical macOS qualification; the build and automated
checks are recorded in [the API evidence](../../docs/evidence/editor-range-api-och41.md).


### Native window lifecycle

`python3 scripts/test_macos_window_lifecycle.py --output <new-directory>` checks
actual minimize/restore and retained native editing. Add `--custom-chrome` for
title-bar drag/fullscreen, AppKit border resize and double-click behavior using
the existing OS preference. The test does not change desktop preferences.
[Physical results and boundaries](../../docs/evidence/window-lifecycle-och41.md)
record both modes, native state acknowledgements and cleanup.


### Independent document-profile scrolling

On Markdown & code, enable **Native document profile**. The review checklist
below the inline/block examples has its own viewport: use the wheel or focus
**Show review start/end** and press Enter. Tab reaches **Open scroll review**
when it is visible and leaves the reader when that control is clipped.
The action uses the existing typed `Open_card` event; scrolling stays native.
Property updates preserve the mounted offset; disabling/remounting the profile
resets it. [Evidence](../../docs/evidence/document-profile-scroll-och41.md#public-scroll-profile--2026-10-05).

The chart page also includes [typed categorical samples](../charts/samples/categorical.md)
with native point/band layout, equal labels with distinct IDs and missing values.

The [stacked chart sample](../charts/samples/stacked.md) demonstrates aligned bars/areas, signed
values, missing observations and a Grouped/Stacked presentation toggle.

The [ordinal color sample](../charts/samples/ordinal_colors.md) demonstrates stable typed slice colors
under source reordering and an explicit unknown-key policy.

[Chart inspection presets](../charts/samples/inspection.md) explain native card, crosshair and marker
configuration and their separation from Bonsai state and scoped source ownership.

**Charts & data → Flow styling** demonstrates the public Sankey presentation
options. The [adjacent sample walkthrough](../charts/samples/sankey_presentation.md)
explains typed node/edge IDs, option-only changes, tiny/zero flows, styled multiline
labels and native selection versus raw values. It traces the actual preset button
through Bonsai state and the GPUIO configuration APIs. The [page guide](charts_page.md) connects it to
Bonsai state and the scoped source owner.

The adjacent [Canvas page walkthrough](canvas_page.md),
[Assets page walkthrough](assets_page.md), [Documents page walkthrough](documents_page.md)
and [Charts page walkthrough](charts_page.md) explain the actual source owners,
Bonsai controls, native observations and page-scope cleanup, with concrete
interaction traces and adaptation examples.

Supporting pure-model walkthroughs explain the [canvas scene](model/canvas_study.md),
[diff expansion reducer](model/diff_state.md) and [raster fixture](image_samples.md),
including their distinction from the Bonsai graphs and scoped source owners.

Desktop and persistence guides explain the [Desktop page](desktop_page.md),
[application-owned desktop session](desktop_session.md), [Settings preview](settings_preview.md)
and [Eio settings writer and expect tests](files/settings_file.md).
The [Extensions page](extensions_page.md) traces the packaged counter's typed
OCaml interface and native lifetime.

Read the [message-slot composition](chat_composition_preview.md) and
[managed transcript](chat_list_preview.md) guides for document/editor lifetimes.
The [popup placement](placement_preview.md), [searchable list](selectable_preview.md),
[controlled toolbar selection](selection_preview.md) and
[window text selection](window_selection_preview.md) guides trace native requests
and application state separately.

Page guides also cover [Find & highlight](highlight_page.md),
[captured input and transfers](input_page.md), [Motion & rhythm](motion_page.md),
[Numbers & codes](numeric_page.md), [input observations](observations_page.md) and
[responsive layouts](responsive_page.md). Each follows the implementation's state,
native ownership and event flow, including diagnostic and platform limits.


The [Charts & data walkthrough](charts_page.md#radar-projection-controls) explains
Radar's shared maximum, fixed radius and label spacing controls, including why
the original-data table and selection values remain unchanged.
