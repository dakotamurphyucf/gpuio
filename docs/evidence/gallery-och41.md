# Component Studio implementation evidence

Status: partial OCH-41 implementation. The gallery and catalog audit
are not complete. This record is narrower than milestone 07 release acceptance.
Local platform: macOS 14.5 (23F79), arm64. Use the pinned repository environment.

The Documents section now includes multi-file diff controls, language syntax,
managed/controlled expansion, preview/Show more, word policy and streamed file
additions. The focused document driver passes on both the repository executable
and an independently built consumer of staged public packages. See the
[diff evidence](diff-controls-och41.md#filename-syntax-word-groups-and-public-gallery)
for exact state, keyboard/AX and lifecycle assertions. This advances the focused
section; the latest combined result is recorded below. Release and Linux desktop
acceptance remain separate.

The [rich-link checkpoint](document-accessibility-och17.md#rich-links-across-rendered-fragments--2026-09-29)
adds public inline-code/bold and safe image-placeholder links, with exact AX
reading-order/action checks and retained keyboard navigation. Its focused result
is separate from the combined/installed-consumer checkpoints below.

## Settings page — focused checkpoint, 2026-09-30

The new twenty-fourth page uses public Settings/Core/Bonsai/Eio composition and
application-owned data/controllers outside virtual rows. The repository driver
passes actual keyboard text/numeric editing, responsive native identity, four
variants, page/filter draft restoration, numeric Cancel to committed value,
guarded text reset, injected task failure and native export-dialog cancellation
with retry. A fresh installed-library consumer also passes these checks, model 249 keyboard
selection/reset and Unicode clipboard paste. Model and real Eio file-replacement
expect tests and the full Dune build/expect/format suite pass separately. The
expanded repository and fresh installed-consumer runs also pass group reset
isolation, filtered-out text and
numeric resets/remounts, blank export rejection, and native Save/Eio readback.
Placement callbacks are fenced by acknowledged row visits; deterministic tests
reject retired/older replies and captured group callbacks after retirement.
The expanded repository and installed-consumer runs also pass physical split/keyboard resizing,
custom-policy reset exclusions, focused pin/eviction/remount across 48 groups and
two-window OS edits/reset/close isolation. Its initial second-window AX focus
mismatch is now repaired with a scoped adapter initialization patch and
[initial/switch/inactive-peer regression](window-accessibility-och17.md); disabled Inert rows
are absent from AX rather than exposed as disabled controls. Those accessibility
limits and full composition/field/style coverage remain open. A native warm-row
retention fix preserves neighbours when the sole visible row is pinned.
The [Settings contract](../design/settings-composition.md#public-gallery-integration-checkpoint--2026-09-30)
records the exact scope and unfinished reset/native requirements. This
new focused check does not extend the earlier complete 23-section result into a
complete 24-section or release acceptance claim.

## Live binding observations — focused checkpoint, 2026-09-30

The Presentation card **Shortcuts in context** uses only public Core/Bonsai/Eio
APIs for live Focused/Editor/Here/Native_context queries. It displays returned
registry shortcuts and native strokes independently of command registration.
Changing or hiding the query preserves the editor's native identity and draft.

Both the repository application and a fresh installed-library consumer pass
20 binding/name/identity cases, actual OS Copy/clipboard and enabled/disabled/
unregistered/replaced-chord invocation, hypothetical editor queries while a
button is focused, invalid native facts/recovery, config epoch reset, unchanged
epoch silence, query retirement and page remount. Caller invocation state survives
page departure; the old native editor does not. Both themes are exercised; the
dark screenshot was visually inspected. No pixel comparison is claimed here.

The [observer contract](../design/command-binding-observations.md#public-gallery)
records exact commands and remaining acceptance. Both bounded GUI processes
exit successfully and are reaped. Full Dune build/tests/format and the structural
catalog audit pass. This section is wired into `core`/`all`; a fresh combined
gallery run, hosted CI and OCH-17 release qualification remain separate. The
subsequent [expanded native matrix](../design/command-binding-observations.md#expanded-native-matrix--2026-09-30)
also passes nested/modal contexts, retained rows, workload limits and independent
windows. Together these establish local functional equivalence for the Kbd row.

## Rich tags — focused checkpoint, 2026-09-30

The Presentation card **Small details, useful actions** uses `Presentation.Tag`:
direct rich children, seven semantic/custom palettes, outline, size groups,
rounded corners, reorder and native hover override/unset. The [contract and
commands](../design/presentation-tags.md) document source mapping and paint-layer
semantics, preserving the original helpers.

The complete native result is from a fresh installed-library consumer on macOS
14.5 arm64: 28 theme/palette/outline GPU cases, eight size measurements, default/
overridden/unset pointer-hover opacity, rich-only/empty content, reorder geometry,
30 OS Return/Space actions, retained focus/identity and slot/page retirement.
Caller state survives remount and registered source bytes reach zero on departure.
The bounded process exits 0 and is reaped, reporting `GALLERY_TAG_OK` and
`GPUIO_GALLERY_AX_OK: section=tags`. Its light screenshot was reviewed. The earlier
repository-native run is only partial evidence; its opacity reference was corrected
without a renderer change or weaker tolerance.

Core palette/style/lifecycle tests and full Dune build/tests/format pass.
The driver is part of `all`/`core`; this
checkpoint does not claim a new whole-gallery run, hosted CI, VoiceOver, Linux GUI,
application performance or clean-machine distribution.

## Rich alerts — focused checkpoint, 2026-09-30

The Presentation page's **A clear next step** card exposes the public
`Presentation.Alert` variants, sizes, title/icon/close slots, Card/Banner,
style overrides, narrow width and caller-owned dismissal. The [contract and
commands](../design/presentation-alerts.md) record precise source differences,
validation, native behavior and scope.

The normal application and independently installed consumer pass 20 theme/variant/
banner cases, eight size layouts, title single-line bounds, wrapped body growth,
retained native control identity/focus, 21 OS body actions, disabled close and
Space/pointer dismissal. Close-slot removal, hiding and page departure retire
controls without losing caller counters. The consumer also passes actual GPU
background/border samples and custom border refinement. Both bounded processes
exit successfully and are reaped. Final markers: `GALLERY_ALERT_OK` and
`GPUIO_GALLERY_AX_OK: section=alerts`.

Full Dune build/tests/format and structural checks pass. No Rust/protocol/fork
change is involved. This establishes local macOS functional equivalence for the
source row; it does not establish a fresh whole-gallery run, hosted CI, VoiceOver,
Linux GUI, application performance or clean-machine distribution.

## Rich markers — focused checkpoint, 2026-09-30

The Presentation page's **Signals that stay out of the way** card now showcases
`Presentation.Marker`: Plain/Separator/Border, Spinner/Shimmer, typed and empty
icons/text, rich controls, custom opacity/lines and compact width. The
[contract](../design/presentation-markers.md) records source mapping, bounds,
identity, explicit accessibility and native ownership.

The repository walkthrough passes 18 theme/variant/icon combinations and 20 OS
Return/Space actions. A fresh installed consumer adds text-only shimmer and rich-
slot removal/reinsertion, for 21 actions. Both check retained focus/native identity,
actual glyph/rich/static pixels, default/custom divider and border paint, reduced
motion, recovery and page retirement. The consumer retains caller state while
replacing disposed native controls; registered source bytes reach zero on departure.
Both bounded runs pass and close/reap their windows. The final driver reports
`GALLERY_MARKER_OK` and `GPUIO_GALLERY_AX_OK: section=markers`.

Full Dune build/tests/format pass; the final gallery checkbox extension was
subsequently built/formatted and validated in the consumer. The latter uses a
separate installed prefix/backend lockfile with the existing toolchain and native
sources, not a clean machine. The focused driver is wired into `all`/`core`, but
this checkpoint does not claim a new complete-gallery run, hosted CI, VoiceOver,
IME, Linux GUI, application-performance or distribution acceptance.

## Rich attachments — focused checkpoint, 2026-09-30

`Presentation.Attachment` adds explicit status, size, orientation, keyed content,
media, actions and a named card trigger. The existing string helper is unchanged.
The [contract](../design/presentation-attachments.md) records application-owned
I/O/tasks/assets, native shimmer inheritance, interaction layering, semantic color
opacity and the pinned source mapping.

The repository's normal-launch `--section attachments` walkthrough passes
20 combinations (two application themes × two axes × five statuses), 21 actual
Return/Space activations and 20 independent pointer Save actions. Each case checks
card/trigger geometry and retained native control identities; real clicks in the
action gap and on the disabled action leave the card count unchanged. Five
horizontal decoded-image sizes and a width-derived vertical square pass. Explicit
styles, optional content/media/action removal, disabled-trigger recovery and page
departure/remount also pass. The native window closes and its process is reaped.
The captured dark vertical card was visually inspected.

The final preview also includes an intentionally invalid image and recovery.
A fresh independently installed consumer passes the same matrix, plus that
decode-failure/recovery case, direct card pointer and AX activation (23 total card
activations), and zero image/registered-source counts after page departure.
The extended result is `GALLERY_ATTACHMENT_OK`, followed by
`GPUIO_GALLERY_AX_OK: section=attachments` and exit 0. The test closes/reaps the
consumer window. Both runs use a bounded 480-second process-group wrapper.

Validation commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
python3 scripts/test_gallery.py --section attachments
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-m7-attachment-consumer-20260930
python3 scripts/test_gallery.py --section attachments \
  --executable /private/tmp/gpuio-m7-attachment-consumer-20260930/consumer/_build/default/main.exe
python3 scripts/audit_component_catalog.py
```

The full Dune build/tests/format checks and fresh installed-consumer build pass.
Core expect tests include 100 status/theme/axis/size transitions, retained owners,
current callbacks, removed-generation rejection, empty-theme operation, image-only
dimming, shimmer override precedence, slot styles and bounded token-aware opacity.
One test draft attempted pointer shielding in an interaction state; shared Style
validation already rejects this. The final test verifies that rule and forced
base shielding without adding redundant state declarations.

The subsequent paint/scroll checkpoint below completes the scoped Attachment
source review. Whole-gallery/release gates and measured application performance
remain open. No Linux GUI, real screen-reader/IME, hosted CI or clean-machine
packaged-distribution acceptance is claimed. The independent
consumer uses an isolated installed prefix and its own backend lockfile, while
still building against this checkout's native sources/toolchain.

### Attachment paint and native scrolling

The new `--section attachment-paint` walkthrough passes against a second fresh
installed consumer in
`/private/tmp/gpuio-m7-attachment-paint-consumer-20260930`. Both themes have actual
GPU-capture evidence for Uploading/Processing glyph animation, static completed/
failed title restoration, image alpha, undimmed overlay, failed-description and
no-image tints, and dashed borders. Native reduced-motion and full-motion recovery
pass. A 280px-wide three-card group passes real horizontal wheel input over an
action gap, clipped-card reveal and activation, owner retention, offset restoration
and vertical wheel propagation to the outer page. Native image/source counters
return to zero on departure; the app closes and is reaped. Final markers:
`GALLERY_ATTACHMENT_PAINT_OK` and `GPUIO_GALLERY_AX_OK: section=attachment-paint`.

The initial dark capture exposed identical default title/highlight colors. The
built-in dark appearance now supplies a white highlight; custom configuration
remains explicit. A separate harness correction replaces an incorrectly declared
variadic macOS wheel call with `CGEventCreateScrollWheelEvent2`; OS field readback
checks both deltas before posting. Moving full AX tree queries out of the capture
loop avoids sampling-period aliasing. Final retained-reference captures observe
48–318 changed glyph samples with roughly 0.4-second sample intervals; the static
and geometric assertions remain unchanged. Earlier failed runs are not full-pass
evidence. These sample counts establish visible motion, not frame-rate performance.

Full Dune build/tests/formatting and the fresh installed-consumer build pass.
See [the Attachment contract](../design/presentation-attachments.md#paint-motion-and-scroll-acceptance--2026-09-30)
for commands, exact scope and intentional source/default differences. The scoped
source row is now a locally validated functional equivalent. The other catalog,
platform and release acceptance requirements remain in force.

## Group boxes — focused checkpoint, 2026-09-29

`Presentation.group_box` now supplies `Group_variant.Card/Plain/Filled/Outline`
and independent header/body/footer wrapper styles. Card preserves the existing
outer-panel default; the other variants place the title/footer outside the body
panel. No new native controller, protocol, dependency or fork patch is required.
The **Structure with flexibility** gallery card uses only public Core/Bonsai APIs.

The combined native `--section core` run passes **64 group configurations**
(two palettes × four variants × two style settings × four header/footer states).
Actual AX bounds verify body padding, outside header/footer placement and separate
slot refinements. A real Space key checks the body checkbox; its checked value and
native identity survive every configuration. **128 real pointer/Return actions**
activate header/footer and body controls with focus checks. Page departure removes
the native subtree and returns image/chart/canvas/source-byte counters to zero;
remount restores the retained caller-owned Bonsai model. The test closes/reaps its
window. Marker: `GALLERY_GROUP_OK`, followed by `GPUIO_GALLERY_AX_OK: section=core`.
Dark-outline and light-filled screenshots were visually inspected.

This combined run also passes the existing 48 status-region, 36 badge/72-input and
48 label configurations, ordinary editing/validation/submission, size/theme
changes, second-window isolation and repeated page transitions. The complete
isolated `dune build -j2 @all @runtest @fmt` passes, including a new group expect
regression covering stable control identity/current callbacks, retired slot
actions, default Card compatibility, idle repeat commits and full unmount fencing.

Two harness assumptions were corrected: checkbox AXValue is a Boolean rather
than a CFNumber; scalar Bonsai page models persist across inactive branches, so
remount must retain their values rather than reset them. Neither is a product
regression. The first expect-test draft also incorrectly rejected newly created
header/footer button configuration as a body-control reset; the final assertion
checks the retained body ID specifically.

The [pinned group-box review](../catalog/presentation-review.md#group-box-functional-equivalent)
records builder/style/ownership mappings and intentional defaults. Thirteen other
presentation modules still need detailed review. Full 23-section reruns, fresh
installed consumers, hosted required checks, Linux desktop, VoiceOver/IME,
performance and clean-machine release acceptance are not established here.

## Enhanced labels — focused checkpoint, 2026-09-29

The public Presentation card now exercises `Label.create` and
`Presentation.styled_label`: inline secondary text, scalar-aware prefix/all-match
foregrounds and display masking in one selectable text flow. The focused driver
passes 48 combinations (two palettes, two widths, secondary on/off, three match
modes and masked/unmasked). Each case sends real Command+A/C and compares exact
clipboard text, checks stable native identity, and verifies that masked AX no
longer exposes the original complete label. Page teardown returns resource counts
to zero. Light/dark and masked screenshots were inspected. The driver pins the
clipboard commands to UTF-8 and restores prior clipboard text.

The combined `--section core` walkthrough also passes with the label checks in
place: status-bar and overlay-badge geometry/input, label wrapping/copy/masking,
editor behavior, independent windows and repeated page navigation all retain their
assertions. The full Dune `@all @runtest @fmt` check passes. The current changes
are OCaml compositions and gallery/driver/docs; they add no Rust or fork patch.

Five new Core expect tests cover matching, expanded Unicode offsets, overlap and
secondary precedence, source/query/output/run limits, masked serialized contents,
and public helper/theme/reconciliation behavior. See the
[pinned label review](../catalog/presentation-review.md#label-functional-equivalent)
for the contract, differences from upstream and reproduction commands.
This focused evidence does not replace the full 23-section/installed-consumer,
IME/VoiceOver, performance, Linux desktop or hosted release gates.

## Latest combined checkpoint — 2026-09-29

Both the repository application and a fresh independent consumer of the staged
public libraries pass the complete **23-section** macOS walkthrough. Both runs
exit normally with `GPUIO_GALLERY_AX_OK: section=all`, exact extension lifetime
verification and transfer gesture identity verification. This supersedes the
older combined-page counts and pending gallery-consumer notes below; those
sections preserve the earlier implementation evidence.

The installed-consumer checkpoint is `f9d125a`. The subsequent
[heading/table accessibility change](document-accessibility-och17.md#heading-levels-and-table-structure--2026-09-29)
also passes the full repository walkthrough, now including table identities,
Unicode cell order and heading level checks across repeated document remounts.
Its independent-consumer rerun remains part of consolidated release validation.

The run includes styling details, input observations, document diff controls and
Find & highlight, along with the existing component, desktop, independent-window
and shutdown checks. The consumer uses its own Dune project and independently
locked native backend against installed public OCaml packages. It uses this
checkout's pinned native sources/toolchain: it is not a clean-machine or packaged
distribution result. Broader family behavior, performance/resource budgets,
screen-reader acceptance and required hosted checks remain open.

The build exposed missing `gpui-base -> unicode-bidi` dependency edges in both
independent application lockfiles. Adding those edges changes no package versions;
the root lockfile already contained the dependency. A first combined attempt
also exposed a driver assumption that the theme was Dark. Earlier sections
correctly retain Light, so the input-observation check now exercises both themes
and restores whichever was active. Both complete runs pass after that correction;
the failed attempt is not counted as acceptance.

Commands for this checkpoint (consumer workspaces must be fresh):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe test/gallery -j2
./scripts/gpuio exec dune runtest test/gallery -j2
python3 scripts/test_gallery.py --section all --images scratch/gallery-direct-images
GPUIO_JOBS=2 ./scripts/gpuio exec python3 scripts/test_extension_consumer.py \
  --example gallery --workspace scratch/gallery-consumer
python3 scripts/test_gallery.py --section all \
  --executable scratch/gallery-consumer/consumer/_build/default/main.exe \
  --images scratch/gallery-consumer-images
python3 scripts/audit_component_catalog.py
```

All pass locally on macOS 14.5 arm64 with the pinned repository toolchain.
The GUI drivers ran under a 900-second process-group watchdog and reaped their
children. Logs and screenshots are retained under the implementing agent's ignored
notepad directory with the `gallery-consolidated` prefix. No new Linux GUI or
physical IME/screen-reader claim follows from this walkthrough.

## Implemented preview sections

| Section | Native/public behavior exercised | Remaining coverage examples |
| --- | --- | --- |
| Presentation | Public compositions, avatar fallback/image semantics, loading controls; screenshot inspected | Image success/failure and complete theme/scale matrix |
| Selection & actions | Button event into Bonsai; native checkbox/switch/radio/select/combobox previews | Full combination of disabled, keyboard and style states in gallery |
| Text editing | OS typing and Enter submit; form validation metadata, theme and preview-size changes retain native text | Real IME/clipboard acceptance remains in release audit |
| Numbers & codes | Single/range sliders, number stepper, OTP, rating; native slider AX increment reaches observed value | Gallery OTP/stepper/rating OS interaction and all scales |
| Dates & colors | Calendar/range and inline color previews; popup date/color changes cancel or confirm correctly; Escape and focus restoration | All constraints/disabled/read-only/scale combinations |
| Overlays & help | Dialog, drawer, confirmation, popover, tooltip and hover-card previews; native dismiss/confirm/focus checks | Tooltip/hover-card keyboard/hover and complete style/scale matrix |
| Navigation & layout | Retained native editor tabs, split, accordion, breadcrumb and pagination; tab text retained/hidden semantics; ordered pagination requests | Split pointer/keyboard and all style/scale combinations |
| Commands & feedback | Command button, popup/in-window menus, disabled semantics, OS shortcuts and chooser selection; progress stages; native toast close/expiry and page departure cleanup | Context-menu/nested-menu keyboard and all theme/scale combinations |
| Carousels & journeys | Horizontal/vertical carousel and navigation-history transitions preserve native edits; hidden editor/links leave AX; icon/offcanvas sidebar keeps selection | Pointer gestures, timed auto-advance and all scale combinations in gallery |
| Lists, trees & tables | 1,000 variable-height entries, far reveal and growth elsewhere; loaded hierarchy reveal/selection; 1,000-row table selection/reveal; native AX row/cell limits; retained internal panels | Paging/retry, sort/resize/reorder, drag and full performance workloads |
| Markdown & code | Scoped Markdown/code/diff, bounded append/reset, parsed code-block controls, native collapse/expand and repeated departure/remount; AX body text/heading/list and read-only code/diff values | Complete screen-reader/selection and rich/image-link semantics, source/selection clipboard and complete theme/scale matrix |
| Find & highlight | Public ordinary/selectable/Markdown scope, styled-leaf match, nested exclusion and Unicode range; native query edits, case/word toggles, counts, selected match, growth/collapse and remount | Broad script/bidi/scrolling, all scales, performance/resource and consolidated release gates |
| Styling details | Clip/start/end ellipsis at wide/narrow sizes, all typed cursor choices, keyboard activation, retained choices across themes/sizes/revisits | Physical OS cursor artwork, specialized-root behavior and broader typography acceptance |
| Canvas & drawing | Validated scene model, native selection/movement/activation, viewport commands, hide/disabled state, reset and scope cleanup | Larger scenes, additional drawing primitives and resource budgets |
| Images & icons | SVG/raster decode, all fit controls, explicit failure/recovery, icon/button semantics and page-scoped registrations | Remaining codec families and full GPU/cache budgets |
| Charts & data | Seven native families plus mixed layers; keyboard selection, publication updates, bounded original-data browsing, disabled metadata, theme/scale retention and scope cleanup | Larger/reduced datasets, pointer selection and release performance budgets |
| Motion & rhythm | Interrupted native targets, tween/spring sequence, pause/resume/cancel/reverse/restart, shared-clock join, live reduced motion and cross-window policy | Full timing/property combinations and resource/performance budgets |
| Native extensions | Independently packaged counter, native AX/keyboard activation, property updates, commands while disabled/hidden, generation reset and exact teardown traces | Additional independent packages, failure/overload gallery fixtures and release consumer gates |
| Input & transfers | Real captured pointer beyond bounds, Escape/disable/page departure, keyboard alternatives, text/custom drag-drop IDs, rejection and retained page state | Gallery OS file arrival and broader generic hover/key/focus/wheel surface audit |
| Input observations | Pointer routing/occlusion, focus, native typing, raw keys, configuration updates, disabled routing, themes, independent windows and departure cleanup | Full native event matrix, physical IME and release resource budgets |
| Responsive layouts | Width/height boundaries, first-match priority, retained branch drafts/counters, silent same-branch resize, hidden AX/focus fencing and theme/size changes | Nested layouts, physical display movement and larger query workloads |
| Desktop services | Shared identity/link/notification receivers, per-window metadata, actual packaged OS links, picker/open/reveal, notification permission/presentation/replacement/action/dismissal | Gallery default-handler reassignment and release packaging/clean-machine gates |
| Runtime & windows | Public diagnostics, observed content geometry, native file picker cancellation | Broader window command matrix |

The earlier combined checkpoint covers 23 sections. The gallery now has 24, with
Settings validated separately as recorded above.
See the [highlighting evidence](subtree-highlighting-och41.md) for the focused
cases and inline-code theme correction.

The combined native test opens a second independent window, verifies independent
editor values, closes it, cycles editor page unmount/remount three times and
closes the primary window. These tests do not yet prove complete resource/cache
budgets or all component families. They use actual macOS accessibility actions
and OS keyboard delivery to the child process, not bridge-injected click events.

## Commands and results

```sh
./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec dune runtest test/gallery
./scripts/gpuio exec dune build @fmt
python3 scripts/audit_component_catalog.py
python3 scripts/test_gallery.py --section all --images scratch/gallery-images
```

All pass locally for this checkpoint. The expect test verifies latest-state
read-only gating of delayed rating requests, saturated relative bursts and
clear/toggle behavior. A second reducer test verifies current command availability,
bounded notification replacement, stale dismissal rejection and departure cleanup.
The combined native test reports
`GPUIO_GALLERY_AX_OK: section=all, native actions, state semantics, focus and shutdown`.
Focused picker/overlay/navigation runs also pass. The shell's preview sizing is
logical sizing, not a substitute for native OS display-scale acceptance.

Initial fixture failures were corrected without relaxing behavior: an inactive
background app did not expose its window through AX; native acceptance launches
normally. Multiline editors use AXTextArea, not AXTextField. The combined test
raises the intended first window and waits for editor focus before typing after
a secondary window closes. Every test child is terminated/reaped on failure.
No passing claim is based on the failed attempts or a screenshot alone.
The first feedback fixture incorrectly expected Bonsai sample state to reset on
page departure. The corrected contract preserves that state separately from native
leases, while lifecycle hooks clear transient choosers, modals and notifications.
The corrected focused and combined runs pass, including departure with a live toast.
Later fixtures await published command availability and sidebar collapse state
before testing enabled/hidden semantics; AXPress queues an asynchronous request.
The native file picker is cancelled through its actual AX Cancel button; a raw
Escape posted immediately after presentation did not close that panel, so it is
not reported as a passing native-picker keyboard test.

Managed previews cap active list rows at 24, tree rows at 16 and table rows/cells
at 24/72. The AX table includes one header row and three header cells in addition
to that budget. The test counts the native tree, observes a last-row cell selection
and preserves the outline selection while switching retained internal panels.
This 1,000-row gallery fixture does not replace release performance/retention tests.

Document registration uses a fresh window child scope on each page activation;
failure closes partial acquisitions and departure suppresses late completions.
After repeated document visits, the public diagnostics page observes zero document
registrations and zero registered source bytes with one window remaining. Those
are application registry counts, not native/GPU allocations or process RSS.

## Open document accessibility finding

The initial external AX walk exposed the document group and toolbar but omitted
the visibly rendered Markdown body. The [OCH-17 repair](document-accessibility-och17.md)
now exposes actual body text, headings/lists and read-only code/diff values.
External native queries, source edit rejection, streaming and collapse/remount
pass. Complete link, selected-text/range, table, heading-level and screen-reader
behavior remains open; body presence alone does not complete accessibility.

Screenshot review also found Markdown links inheriting an unreadable default
color on dark backgrounds. The native document adapter now supplies explicit
light/dark link colors. Native document regression and visual checks accompany
that correction; link contrast alone does not establish accessible link behavior.

## Catalog audit boundaries

The immutable source snapshots verify by SHA-256. The structural inventory covers
67 base and 79 component root modules, 73 GPUIX style fields, 22 generic events,
12 intrinsic elements and 13 React export modules. `families.json` assigns every
root module exactly once to 43 owning families and links interfaces/examples/
evidence; `gpuix-styles.json` maps all style names to existing properties/states.
The script rejects missing/duplicate modules, missing API properties and broken
local references. It does not infer behavior from source names.

Detailed family/configuration/event/value review remains explicitly pending.
Known review topics include GPUIX's previously omitted onVisibleRange/onHighlight,
ellipsis-start and extra cursor variants, pointer/wheel defaults, selection scope,
standalone clipboard/automation surfaces and nested editor/document plugin APIs.
The gallery now includes desktop services. The input/transfer page
covers existing captured-pointer and drag/drop APIs; broader generic-event
functionality still requires the detailed GPUIX audit. OCH-41 remains In Progress; OCH-17's macOS release audit follows it.

Linux build/unit/private-bus/consumer checks remain required. This checkpoint
adds no real Linux GUI qualification; OCH-47 owns that deferred work. Hosted
acceptance for this implementation branch has not yet been run.

## Motion gallery continuation

The thirteenth page uses only `View.animate`, `View.animate_program` and the public
application motion policy. System is the initial preference; an explicit
System/Reduced/Full selection updates all gallery windows through shared Bonsai
state and `App.set_motion`. No animation-frame OCaml timer is installed. The
preview keeps only the latest bounded native observation batch. Page departure
unmounts wrappers, pauses the sequence model and stops the repeated preview;
returning does not resume the previous repeating work automatically.

The focused local macOS motion test passes actual native intermediate width
measurements, reversal from the painted position, held geometry after pause,
requested cancellation, replay and reversed endpoints. Reduced motion reaches
endpoints without intermediate widths and reports the three reduced stages. A
new window reflects the shared preference and changes it for the first window.
Dynamically joined members share phase; Reduce freezes them at initial values.
Leaving/remounting removes old accessible samples and restores stopped playback.
The native test measures AX layout geometry, not physical display presentation
timestamps or a frame-rate/idle-memory budget.

The initial fixture wrongly assumed AX tree traversal plus input would pause
within the first 550 ms stage. It reached the spring stage instead and correctly
held there. The corrected test requires prior movement, stable paused geometry
and a value distinct from the final endpoint; it accepts a pause in any active
stage. The original failure is not counted as acceptance.

A combined-run fixture also queried departure before the asynchronous page-change
request was applied. It now waits for the destination page before asserting that
the old samples are absent. The focused test's success is separate from that
failed combined attempt. The latest combined result is recorded below.

Latest thirteen-page continuation: gallery build, `dune runtest test/gallery`,
`dune build @fmt`, catalog audit, `git diff --check` and
`python3 scripts/test_gallery.py --section all` pass locally on macOS 14.5 arm64.
The combined run includes OS Space activation of the System preference control,
all document keyboard-link checks, the independent additional window, registration
cleanup and native file-picker cancellation. All child processes are reaped. The
focused motion screenshot was inspected. Required hosted checks and the installed
gallery consumer remain pending; this is not full OCH-41 or OCH-17 acceptance.

## Charts gallery continuation

The fourteenth page covers line, area, bar, pie, radar, candlestick, Sankey and
mixed layers through public OCaml chart APIs. Deterministic sample data is shared
with the standalone Chart Studio in `examples/charts/samples`; its family identity
is a closed variant. The original standalone numeric controls use a small checked
adapter. These example modules own no native resources and are not a new GPUIO API.

Each page activation acquires one fresh scoped chart registration. Family changes
reset its selection epoch; sample updates preserve stable identities. Current
source data resolves native selection descriptions only after publication.
Per-window family/style choices survive departure, while the source phase and
selection reset on reacquisition. Ready messages identify the family and source
value count; Sankey's data companion includes node records as well as numerical
edge values, so its original table row count differs from that metric.

Focused native macOS acceptance passes all eight presentations, Home/Enter
selection through OS keyboard delivery, updated line values, unchanged pie
publication, original-data row counts and End navigation, at most ten mounted
accessible data rows, disabled metadata, theme and all three preview sizes, and
three departure/remount cycles. The runtime snapshot then observes zero chart
registrations and zero registered source bytes. Those are application registry
counts, not total native/GPU memory or leak-budget acceptance. The light-theme
mixed-layer screenshot was inspected.

Initial focus fixtures did not reliably establish the active window before
requesting chart focus; an immediate launch-time activation was also too early
for the OS AX application. App activation alone passed the focused run but failed
after the earlier gallery interactions. The final fixture waits for ready
content, activates the app and explicitly raises the intended window before
focusing the chart, matching the existing editor test. The unchanged focus
assertion and complete fourteen-page walkthrough then pass. Failed attempts are
not counted as native acceptance.

Both gallery and standalone chart builds, gallery expect tests, Dune formatting
and structural catalog checks pass locally. The standalone `--self-test
--foreground` also passes all seven families, reset generations and scoped
release after the sample extraction. No Rust/runtime change or new dependency
pin is part of this continuation. Consolidated hosted/installed-consumer and
release performance gates remain pending.

Final combined continuation: `python3 scripts/test_gallery.py --section all`
passes all fourteen pages on macOS 14.5 arm64, including both chart and motion
keyboard/focus assertions, document links and final native picker cancellation.
`dune build @fmt` and `git diff --check` also pass after the final edits. All test
processes are reaped. This adds gallery coverage, not completion of the detailed
catalog parity ledger, installed-consumer tests or milestone 07 release gates.

## Canvas and image gallery continuation

The fifteenth and sixteenth pages use public drawing/image/icon and scoped Eio
registration APIs. The pure `Canvas_study` model caches a validated scene; a move
validates the whole candidate, including labels, before accepting it. Expect tests
verify independent shape positions, an unknown ID, coordinate overflow, unsupported
text rotation and preservation of the initial model. Native moves update source
transforms without applying movement twice. Activation is displayed independently
from transient movement/command status. A page visit owns one scene; hide/show
retains it, while departure reacquires a fresh scene and clears transient state.

Native canvas checks cover repeated OS Shift+Arrow movement with measured bounds,
activation, 125% viewport geometry and reset, selection commands while user input
is disabled, retained hide/show positions, source reset, both appearances, all
three preview sizes and three page visits. Optional `--trace-canvas` records
public events with publication/window snapshots, including monotonically ordered
command acknowledgements and fresh scene/command state after reacquisition.

Early canvas fixtures lost an expected movement/activation observation; later AX
window lookup also failed. Failed logs are retained, without asserting a proven
native renderer defect or claiming those runs passed. The final test establishes
native readiness and raises the intended window between consecutive moves.
A final focused regression asserts that object focus survives each source
publication without refocusing the object.
The observed event trace and full combined walkthrough pass with those explicit
input preconditions. A trace-variable syntax error (`effect` is reserved in this
compiler) was corrected before the final build; the accidentally reused prior
binary run during that failed build is not acceptance evidence.

The image page owns four in-memory registrations: an authored SVG landscape, a
96×48 PNM gradient, a deliberately malformed PNM source and an SVG icon. No file or
network acquisition is implicit. All fit controls preserve original raster
metadata; invalid decoding reports `Invalid_data` and switching back recovers.
Standalone image/icon semantics and actual OS Space/button activation pass.
Appearance/size changes preserve ready data. Three page visits return image,
canvas and registered-source-byte counts to zero. These counts exclude native
caches/GPU allocations and do not replace release memory budgets. Screenshots
of the canvas and vector/raster/icon compositions were inspected.

Local macOS 14.5 arm64 checks pass: gallery build, gallery expect tests, Dune
formatting, structural catalog audit and the combined sixteen-page native
walkthrough. No Rust/runtime or dependency pins changed. Hosted checks, installed
gallery consumers, detailed parity and complete release gates remain pending.
The catalog now also records five accepted v1 additions outside the pinned
root-module map; structural validation is explicitly separate from behavior.

After the combined run, additional focused checks also pass: real macOS pointer
drag moves Orbit from (110,130) to (134,146), with matching native bounds and one
accepted source transform; consecutive keyboard moves retain object focus across
publication without refocusing. The image page exposes exactly three meaningful
AX images; the button decoration introduces no duplicate accessible image. These
final assertions were validated in focused canvas/assets runs. All processes are
reaped, and the working copy's format/expect/catalog/whitespace checks pass.

## Responsive gallery continuation

The seventeenth page uses the public `View.container_query` API. Explicit logical
sizes make the rule order and half-open boundaries inspectable: below 230 pixels
high selects Short at any width; otherwise width at least 480 selects Wide;
otherwise Compact. The outer size is independent of preview typography scale.
Each supplied Bonsai branch owns its own counter and native editor. All three
branches remain mounted while selecting between them. The wide presentation
places the draft and save action in a row; the other presentations stack them.

The focused macOS run passes initial selection at 400 × 300, a resize to 479
without another observation, transition at exactly 480, height precedence at
200, another silent width change to 600, and transition at exactly 230 tall.
It checks actual editor geometry after the 79-pixel same-branch resize, retained
text and counts on return, hidden-editor absence from AX and a stale reference
unable to focus the hidden editor. Real OS typing updates all three drafts.
Light/dark and all three preview sizes preserve selection and draft state.
Three page departures remove accessible editors; re-entry acquires fresh drafts
while preserving the Bonsai counts and chosen dimensions. Both compact and wide
screenshots were inspected.

`./scripts/gpuio build examples/gallery/main.exe` and
`python3 scripts/test_gallery.py --section responsive --images scratch/gallery-images`
pass locally, along with gallery expect tests, `@fmt` and the structural catalog
audit. This focused continuation follows the earlier sixteen-page combined run;
the subsequent eighteen-page combined run passes as recorded below; hosted checks
remain pending. No native
runtime or dependency change was needed. This gallery check does not substitute
for the independent native-resize/no-OCaml-commit evidence in
[OCH-26](container-queries-och26.md), physical DPI acceptance or release budgets.

## Native extension gallery continuation

The eighteenth page consumes `Gpuio_example_counter` and the existing generated
`gpuio_counter_backend`. The gallery verifies the linked schema catalog before
opening a window. It uses properties, typed observations, one outstanding
sequenced command, explicit input disabling, retained hiding and native generation
replacement. A bounded immutable controller checks the captured generation of
observations, serializes property/step changes behind pending commands, and clears
pending work on departure. Reset and departure advance generation; pure expect
tests verify mismatched acknowledgements, obsolete generations and pending-command
behavior. Native page revisits restore the current model rather than replaying
an old command.

The focused macOS test passes AX activation, real Space/Enter delivery, updated
step/value properties, disabled input rejection, explicit commands while disabled
and while hidden, hidden accessible-reference rejection, theme/size changes,
generation reset, and three page revisits. A retained OS accessibility reference
resolves the current logical button after generation replacement: its label changes
from 42 to 7 and activation advances the replacement to 12 with step 5. This is
current action resolution, not permission for revoked native callbacks to run.
An earlier test incorrectly required that live logical reference to become inert;
the corrected test checks its current name and the replacement's value.

This integration also found the sample button reporting enabled metadata while
the host correctly blocked disabled input. The host group's disabled state does
not inherit to its child's AccessKit node. The counter package now sets disabled
state on its actual button using the public GPUI accessibility builder and the
current SDK event-lease check. Live callback guards remain unchanged. The focused
native test verifies both disabled metadata and input rejection, then recovery.
No SDK, wire schema, runtime ownership or dependency pin changed.

Opt-in package traces verify exactly five mount/unmount/component-drop/value-drop
lifetimes across initial mount, reset, three revisits and final close. Exactly two
native commands execute; theme/resize/re-render/revisit does not replay them.
The trace checks last callback-value release, not just component object disposal.
This does not measure general GPU/process-memory budgets. Screenshot inspected.

Gallery build/expect tests, Dune/Rust formatting and strict all-target counter
package Clippy pass locally. Focused `--section extensions` and the combined eighteen-page walkthrough pass
with this composed backend. The full run includes the responsive page, document
keyboard/accessibility behavior, canvas pointer movement, charts, motion and
resource cleanup. All test children are reaped. Hosted and installed-consumer
gates remain open.

During combined validation, a Line chart's initial focus request was not accepted
after raising the window. The fixture had sent one request and then waited
without retrying it. The chart's native focus action requires current window
activation and input eligibility; raising a macOS window is asynchronous. Initial
chart setup now uses a bounded request/confirmation loop before sending keys.
The focused run needed two requests for Line, one for each subsequent family,
and passed all original keyboard/selection/data assertions. Focus-retention
checks elsewhere still assert focus without reacquiring it. This changes test
setup, not production chart behavior. The complete cause of earlier reduced-motion
observation/AX-window timeouts remains unverified; focused motion passes unchanged.
`--trace-motion` records semantic batches and the motion fixture explicitly raises
the intended window before foreground rendering checks.

The final combined run passes with both diagnostic traces enabled. Earlier
attempts are not acceptance: one included extra shape drags and viewport panning
outside the scripted sequence, consistent with the owner's report of interacting
with the gallery during testing; others failed initial chart focus or a motion
observation wait. The final run keeps all behavioral assertions, uses the explicit
foreground/initial-focus setup above and verifies terminal extension cleanup.

## Input and transfer gallery continuation

The nineteenth page uses public `View.pointer_area`, `drag_source` and `drop_target`.
It supplies ordinary keyboard-accessible size/receive controls alongside raw mouse
surfaces. Width is bounded to 80–360 logical pixels; latest notices and a saturated
receipt count replace any event history. The inbox accepts text/file metadata and
optionally one application-specific card format. It does not read/open received
paths or decode opaque card data. Native rules decide acceptance before queued
OCaml observations; keyboard alternatives honor the same enabled/format policy.

The focused macOS test posts real mouse events after checking target process
ownership. It verifies capture above the region, measured 260-pixel output width,
release, Escape cancellation, disabling during capture, rejection of fresh input
while disabled, and actual Space activation of size/receive alternatives. It
unmounts the page during a held gesture, sends the late release, then verifies a
fresh gesture succeeds after remount. Three additional page visits preserve width
and received count while clearing transient gesture feedback.

Text and opaque-card transfers each deliver once. Cancellation, a rejected card
format and a disabled source do not increment the count. Opt-in semantic traces
contain exactly four source gestures, with matching source/target identities for
the two accepted native drops; the keyboard alternative is a separate application
action, not a fabricated native drop. Both themes and three preview sizes preserve
state. Screenshot inspected; no native runtime or dependency changes were needed.

Gallery build, gallery expect tests, `@fmt`, Python syntax, structural catalog audit
and focused `--section input` pass. The initial native fixture expected 32 bytes
for a 33-byte UTF-8 greeting; its failed log is excluded, and the corrected test
passes. The combined nineteen-page run and hosted gates remain pending after the
eighteen-page checkpoint above. No general GPU/RSS budget, passive-hover callback,
OS file-arrival or complete generic-event parity claim follows from this preview.
Existing standalone OS file-transfer coverage is recorded separately in
[OCH-11](drag-drop-och11.md).

## Desktop gallery continuation

The twentieth page uses public `App.run_desktop`, `Desktop`, `Notification`,
`File_dialog` and `Desktop_package` APIs. One application-owned service of each
kind survives page/window changes. Only latest observations and one receipt are
retained. An effect-execution guard serializes notification operations across all
windows; terminal events retire only the matching receipt. Actions update the
shared model without automatically activating a window. Document metadata is
observed from each exact native window on page entry. Selecting a represented file
neither reads nor writes it; Open/Reveal, handler registration and notification
permission are separate explicit actions. Native path bytes are not UI labels.

Direct macOS execution passes startup-link delivery, actual edited/saved metadata,
independent windows, shared observations, three page visits, both themes/three
sizes, disabled file actions without a path and native picker cancellation. It
also checks actual `Unavailable` responses for unbundled notification posting and
scheme registration. This is expected platform behavior, not a fallback claiming
that a notification was delivered.

`python3 scripts/test_gallery_desktop_macos.py` builds a disposable ad-hoc signed
bundle using the gallery's generated plist. It passes:

- Actual Launch Services links to the same process, including receipt while the
  desktop page is hidden and rejection of an invalid authority.
- Real AppKit picker selection of a disposable Unicode-named fixture. The
  OS-selected temporary handler records the exact requested path; Finder exposes
  that fixture after Reveal. Clearing the path disables file actions again.
- Actual notification support/authorization. Posting before permission returns
  `Not_ready`; the explicit permission action presents the OS prompt and granting
  it yields `Authorized`. Native Notification Center shows the posted content,
  its replacement and the named action. The action reaches OCaml while the page
  is hidden and clears the matching receipt. A second window posts the next
  receipt; closing that window preserves the receipt so the first can dismiss it.
  The test confirms native disappearance and normal final application shutdown.

The final packaged run passes in `desktop-packaged-ax-4.log` (local scratch).
Earlier failures are excluded: the picker fixture assumed a named combo box and
used process-targeted keys. A focused native diagnostic showed Sonoma exposes an
unnamed text field in a nested sheet and needs the foreground event stream for
its shortcut. The final test checks foreground ownership and waits for the nested
sheet's arrival/departure. A separate run failed to expose the initial AX window;
setup now repeats bounded activation until that first window is available. No
library/runtime changes were made for these fixture fixes.

Gallery build, expect tests, formatting, Python syntax and structural catalog
checks pass. The direct gallery screenshot was inspected. Combined twenty-page
validation passes (`desktop-all-ax-2.log`); hosted review and release gates remain
pending. The packaged fixture does not establish clean-machine distribution,
Developer ID signing/notarization, global accessibility or resource budgets.
Default-handler reassignment is an explicit gallery action but was not exercised
by this packaged fixture; earlier standalone native coverage remains separate.


The first combined twenty-page attempt reached motion but missed its intended
interruption interval: a new accessibility-tree search between contraction and
reversal let the sample approach its narrow endpoint (97 pixels). The revised
fixture resolves the same native sample/toggle before starting the interval;
it retains the original intermediate/endpoint/no-jump assertions. Focused motion
passes with 283 pixels before reversal, a 279-pixel minimum afterward and the
310-pixel final endpoint. The combined rerun passes too (288 pixels before reversal, 279-pixel minimum
and 310-pixel endpoint). This is a fixture change, not a relaxation of the motion contract or a production animation change.


The combined run also verifies exact extension teardown (five mount/unmount/
component/value lifetimes, two commands without replay), matching transfer
identities (four source gestures/two native drops), zero registered source bytes
at the final runtime page, picker cancellation and normal application shutdown.
All required v1 entries in the current root-family map and expanded-capability
map now have a gallery link. The only null root-family links are post-v1 docking
and excluded development-inspector tooling. This is navigation/example coverage;
nested configuration, events, style values and accessibility still require the
explicit behavioral audit and may reveal additional implementation work.

## Styling values and event audit — OCH-41 continuation

The twenty-first page, **Styling details**, uses the public OCaml style API to
compare clipping, end ellipsis and start ellipsis at 250/140 logical pixels. It
cycles all 22 typed cursor choices, with keyboard activation and retained choices
across appearance, three sizes and three page revisits. Complete source text stays
in the accessible labels. Inspected wide/narrow screenshots show actual start and
end truncation in the correct directions. The cursor test checks configuration
and native GPUI refinement, not the physical OS cursor artwork.

The library adds twelve cursor values and `Style.Text_overflow.Ellipsis_start`
without changing existing value IDs. Capability bit 44 rejects incompatible hosts
before those values reach native decoding. An independent Rust/OCaml wire fixture
covers every cursor and overflow value; native session tests reject invalid
negative/upper-bound values atomically, preserving earlier text/revision/memory,
and verify registration cleanup after close.

Local macOS 14.5 arm64 checks passed:

- `./scripts/gpuio build examples/gallery/main.exe`.
- `./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j 2`.
- `./scripts/gpuio exec cargo test -p gpuio-native --locked -j 2`.
- `./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --locked -j 2 -- -D warnings`.
- `./scripts/gpuio exec dune runtest`.
- `python3 scripts/test_gallery.py --section styles --images scratch/agents/root-20260928-m7/images`.

The focused GUI run completed normally and reaped its child. The previous combined
20-page result above predates this addition; it is not a combined 21-page result.
Exact local logs use the `style-values-` prefix in the implementing agent's ignored
notepad directory. The first protocol run correctly detected stale Hello golden
bytes; reviewed capability-mask expectations were updated and the full suite then
passed. These results are not hosted or Linux desktop acceptance.

The catalog now checks all 29 pinned cursor keywords, both ellipsis values and
all 22 event-property audit rows. The event audit distinguishes semantic native
commands/viewport observations from missing generic pointer/key/focus/wheel
observations, and whole-document collapse/search from missing per-file diff and
arbitrary-subtree highlighting controls. These gaps remain required v1 work, not
accepted deferrals. Detailed remaining style values, nested component behavior,
installed consumers and OCH-17 release acceptance remain open.


## Input observations and pointer occlusion — twenty-two-page checkpoint

A public **Input observations** page now wraps an Eio-owned native editor with
Core/Bonsai `View.input_region`. All thirteen event kinds are represented by bounded
counters/latest samples. Listener configuration changes retain the editor; leaving
the page clears observations and releases the editor lease. A floating sibling
button demonstrates explicit pointer occlusion while retaining wheel propagation.

The focused macOS `--section observations` run passes actual pointer down/up/click,
AX focus and native typing, Tab/Shift-Tab order, raw F13 from the retained editor,
capture/bubble configuration changes, disabled/re-enabled observations, the floating
action without underlying pointer delivery, dark/light themes, independent second
windows and page departure/revisit. Both theme screenshots were inspected. The
final local run is `observations-ax-6.log`; earlier failures and fixes are recorded
in the [input evidence](input-observations-och41.md). The harness closes/reaps both
windows. The combined walkthrough now accounts for the extra independent window;
a combined twenty-two-page result is still pending.

Native checks separately cover all buttons/counts/modifiers, precise/line wheel
units and touch phases, nesting/propagation, clipped bounds, native window exit,
modal scopes, deactivation, repeated keys, focus edges, macOS marked-text retention,
all three occlusion modes and foreign capture ownership/disposal. Public
`Style.Pointer_occlusion` appends field 66 and capability bit 46. The API separates
listener eligibility from occlusion; it does not infer blocking from background
color or absolute positioning. The catalog records this deliberate difference.

The final full protocol/native suite passes 683 Rust tests; full OCaml expect
tests, feature-enabled strict lint, formatting and the structural catalog audit
also pass locally. Native input regions are now functionally mapped in the GPUIX event ledger with
these local evidence limits. Remaining subtree highlighting, per-file diff controls,
other style/nested-family audits, independent installed consumers and consolidated
OCH-17 release/hosted gates remain required work. This checkpoint is not Linux GUI
or completed milestone acceptance.
