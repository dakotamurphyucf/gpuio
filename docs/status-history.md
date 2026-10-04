Document resource qualification now covers one shared runtime across 36 window
cycles and 808 virtual-block visits. Replaced source snapshots, native profiles,
callbacks and parser reservations release; every close returns the document service
to zero live resources. Full native **877 passed** (two existing macOS private-bus
skips), strict lint and formatting pass. This is TestPlatform ownership evidence,
not process/GPU memory or physical timing acceptance; see
[resource evidence](evidence/document-resources-och17.md).

Release notice collection now covers the explicitly named OCaml switch and all
eight vendored native Bonsai roots: 144 package/root records and 499 retained
notice texts, including the unchanged OCaml 5.3 runtime license. Fresh native,
Signal and gallery Rust inventories have 28 missing-text packages each. Exact
source provenance resolves zune-inflate, and document SDK/profile/backend mappings
are current. Collector/packaging tests and workflow lint pass; final licensing,
asset/system and distribution review remain open. See [OCaml evidence](evidence/ocaml-notices-och17.md)
and the [Rust follow-up](evidence/dependency-notices-och17.md#current-consumer-graphs-and-exact-source-follow-up--2026-10-04).

Virtual document controls now reveal offscreen blocks and controls within tall
blocks before accepting keyboard focus. Seven new production-host regressions
cover actions, inline/block profiles, reverse traversal, exit, stale requests and
rapid input. Full native **876 passed** (two existing macOS private-bus skips),
Base text/selection **235 passed**, strict lint, gallery build, formatting and
structural catalog checks pass. Both vendor patches reconstruct exactly. Physical
and broader focus/resource qualification remain open; see
[virtual focus evidence](evidence/document-virtual-focus-och41.md).

Application-wide document defaults now flow through both Eio runners, later window
creation and reconciliation. Inherit/Builtin/Value retains per-field intent; local
callbacks/profiles override shared defaults, with explicit profile suppression.
Five Core cases, Eio/window-driver isolation tests, the full OCaml suite, formatting
and an independently installed gallery pass. The opt-in gallery defaults comparison
is implemented; physical layout/copy/accessibility qualification remains open. See
[defaults evidence](evidence/document-defaults-och41.md).

Earlier checkpoints below describe the scope and remaining work at their own revisions.

A public document-profile package now supplies typed OCaml properties/events and
independent Rust highlighting/actions/inline/block plugins. The gallery composes it
with the counter backend and exposes optional native review controls. Rust package
tests, the full OCaml suite, strict lint, six generator tests, formatting and a fresh
installed-gallery consumer pass. That consumer also runs the copied expect test and
checks both native catalogs in a fresh process without opening a window. Application
defaults, virtual focus and physical/release qualification remain open; see
[package evidence](evidence/document-profile-package-och41.md).

Document profiles now install into the native reader: prepared text, plugin
adapters, custom highlighting and native code/table/inline/block renderers share
one installed lifetime. Seven new production TestPlatform cases cover events,
revocation, pointer/keyboard policy, failures, image refresh/selection and retained
resource release. Full native **869 passed** (two existing private-bus skips),
document SDK15 and extension SDK5 pass, as do strict lint, gallery linking, formatting
and the structural catalog audit. Gallery/defaults/independent profile
consumer, virtual focus and physical/release gates remain open; see
[renderer evidence](evidence/document-profile-renderers-och41.md).

# Implementation status

The entries below describe earlier checkpoints and the limitations at each checkpoint.

Document workers now support checked profile requests, unpublished-source waiting,
configuration/parse/highlight failure stages, shared reservations, cancellation and
complete custom glyph styling. Nine new cases pass in the full native862 suite
(two existing private-bus skips); SDK15 and strict lint also pass. Presentation
attachment is still unfinished, so views do not apply profiles yet. Completed pictures release unused work reservations; 128 small profiles fit the
shared budget. Rendering/event leases, defaults/gallery and release gates remain open;
see [worker evidence](evidence/document-profile-workers-och41.md).

Document profiles now have checked Core/Bonsai attachment, paired bounded
Op121/Event78, Eio dispatch, atomic native catalog/admission and backend manifest
composition. SDK15/protocol382, native853 (two existing private-bus skips), native
admission, full OCaml build/tests/format, strict lint, six generator tests and a
fresh installed-gallery consumer pass. A profile-only generated backend compiles.
The native reader does not yet apply profiles: worker/render attachment, resource
reservations, gallery/defaults and physical qualification remain open. See
[bridge evidence](evidence/document-profile-bridge-och41.md).

A static document SDK foundation now defines checked profile/schema bindings,
worker preparation, plugin presentation contracts and native renderer interfaces.
Fifteen SDK tests exercise real Markdown/HTML preparation, source/projection
behavior, cancellation, malformed output and highlight limits. Strict lint,
workspace compilation, formatting and catalog checks pass; only one local crate
was added to the lockfile. Host/protocol/Core/Bonsai/Eio attachment, resource
reservations, application defaults, gallery and native qualification remain open.
This is not yet an application feature; see [SDK evidence](evidence/document-sdk-foundation-och41.md).

Declarative rich-document actions now expose code/table buttons, independent
native Copy controls and queued revisioned snapshots through Core/Bonsai/Eio.
The gallery demonstrates enabled state and payload notices. Native tests found
and fixed host fallback stealing child-button focus; action rows now wrap below
code and remeasure retained views. Protocol380, native852 (two existing private-bus
skips), strict lint, full OCaml checks, formatting and a fresh installed-gallery
consumer pass. Base reconstructs exactly (234 files). Arbitrary static document
renderers/highlighters/defaults, offscreen virtual control traversal qualification
and physical/release gates remain open; see [action evidence](evidence/document-actions-och41.md).

Frontmatter descriptions now expose native label/value rows through public
Markdown options and the Documents gallery, with unsupported YAML retained as
code. Selection, copy, search paint, reflow, semantics and preview checks pass. A
Flow-layout search-paint regression found during testing is fixed. Protocol378,
native847 (two existing private-bus skips), strict lint, full OCaml checks and a
fresh installed-gallery consumer pass; GPUI Base reconstructs exactly (234 files).
Custom document actions/highlighters/defaults/static plugins and physical/release
gates remain open; see [frontmatter evidence](evidence/document-frontmatter-och41.md).

Public Markdown parser options now support YAML frontmatter as code and MDX
syntax, with stable source/native ownership, superseded-worker rejection and
selection/link invalidation when the interpretation changes. A pinned MDX
conversion bug that dropped JSX children is fixed. Protocol378, native840 (two
existing private-bus skips), strict lint, full OCaml checks, formatting and a fresh
installed-gallery consumer pass. GPUI Base reconstructs exactly. The frontmatter
description-list renderer, static document extensions and physical/release gates
remain open; see [parser evidence](evidence/document-markdown-options-och41.md).

Document link callbacks now include input source, mouse button and release
modifiers. Markdown/HTML native tests cover all five buttons, keyboard routing,
source-reset rejection and no automatic URL opening. Protocol377, native836
(two existing private-bus skips), full OCaml checks, strict lint, formatting and a
fresh installed-gallery consumer pass. The public Link payload is a record;
legacy wire metadata stays unknown. Physical and remaining catalog/release gates
stay open; see [link evidence](evidence/document-link-activation-och41.md).

Public `Document.Style` now controls rich Markdown/HTML palettes, heading/paragraph
metrics, inline code and passive code/table/header/cell refinements. Style and theme
changes preserve parsed text and selection while remeasuring native/managed rows.
Protocol376, native833 (two existing private-bus skips), strict lint, full OCaml
checks and a fresh installed-gallery consumer pass. The gallery includes a styling
toggle. Physical and remaining catalog/release acceptance stays open; see
[styling evidence](evidence/document-styling-och41.md).

The public HTML reader now supports bounded worker parsing, registered images,
queued links, plain selection copy and preview observations, with a streaming
HTML gallery example. Protocol375 and native832 pass (two existing private-bus
skips), as do strict lint, full OCaml/expect tests and a fresh installed-gallery
consumer. The GPUI Base fork reconstructs exactly. Physical acceptance and
remaining catalog requirements stay open; see [HTML evidence](evidence/document-html-och41.md).

Public document preview now exposes a validated Markdown Flow line budget,
queued presentation observations and an expandable gallery example. Limit-only
changes retain source/parser/native identity. Protocol374, native826 (two existing
private-bus skips), atomic admission, Core/OCaml/gallery and strict lint pass.
The stronger source-reset/window-resize test and a fresh independent installed
gallery consumer also pass. Physical qualification and other catalog gaps stay
open; see [preview API evidence](evidence/document-preview-api-och41.md).

Native preview clipping prerequisites also align visual, pointer, keyboard/focus
and accessibility behavior; see [their evidence](evidence/document-preview-prerequisites-och41.md).

Document selected-content copy now exposes Plain_text/Markdown through a public
config option and gallery toggle. Op116 updates the retained setting without
republishing the source. A stronger window-copy test found and fixed Markdown
source whitespace trimming; the fork reconstructs exactly (233 files). Full
OCaml/gallery, protocol372, native816 (two existing private-bus skips), atomic
admission, strict lint and a fresh independent gallery consumer pass. Physical keyboard/
clipboard and remaining document catalog/release checks stay open; see
[selection-format evidence](evidence/document-selection-format-och41.md).

The nested display-document audit now maps nine exact upstream sources to the
public API. It identifies remaining reader-format, selection/clamp, internal
styling and static plugin/renderer gaps separately from deferred editor/LSP work.
Existing document behavior is unchanged; this is source evidence, not runtime
acceptance. See [document review](catalog/documents-review.md).

Rust dependency notices now have an offline collector with Cargo identities,
lockfile/metadata provenance and preserved text hashes. Ten portable tests pass.
Explicit pinned workspace/registry attributions cover 46 packages in both native
and Signal Studio inventories; 29 packages in each still lack collected text.
All license, OCaml/asset and distribution approval remains open;
see [notice evidence](evidence/dependency-notices-och17.md).

The redirected-output failure now has a narrow `Gpuio_eio.Output.write` adapter:
blocking character-device writes run through Eio system threads while files/pipes
retain normal flow behavior. Exact output, borrowed-fd/error behavior, cancellation
and scheduler responsiveness pass without GUI windows; five reference exporters
are updated. Full OCaml tests/formatting, the five builds and an independent
installed-gallery build pass. A fresh signed/extracted Agent Workspace bundle
also passes the original `/dev/null` metadata regression. The pinned switch/Eio
sources are unchanged; see [output evidence](evidence/output-sinks-och17.md).

Local reference-app packaging now assembles audited Agent Workspace, Component
Studio and Signal Studio bundles/zips with optional ad-hoc signing and provenance.
All three pass local dependency/minimum/signature/extracted metadata checks.
GPUI/GPUI Base/AccessKit source reconstruction also passes. These are internal
qualification artifacts: clean-machine GUI, transitive notices, release signing
and publication remain open. A metadata export to `/dev/null` exposed an Eio poll
assertion, addressed in the subsequent output adapter above. See [release-input evidence](evidence/release-inputs-och17.md)
and [distribution instructions](distribution.md).

The OCH-17 chart diagnostic now separates native publication, render-dependent
Ready and requested render-callback waits, records focus snapshots without
claiming visibility, and preserves partial/error reports with bounded child cleanup.
Nine offline/non-GUI process tests and the OCaml workload build pass. The original
63-second sample remains unexplained; no new physical performance run is claimed.
See [staged diagnostic evidence](evidence/chart-streaming-och40.md#och-17-diagnostic-stages-2026-10-03-physical-rerun-pending).

Custom managed-table headers now integrate through Core/Bonsai, Op113 admission,
reconciliation and the native retained renderer. A public gallery toggle supplies
leaf/group Views, including a button with independent actions. Header keys survive
content/target updates separately from virtual body rows; mapping changes fence
old native gestures. Native tests (813 library/12 table admission), protocol tests
(369), full Core/Bonsai/gallery checks, strict lint and an independent installed
public-gallery build pass. Two existing private-D-Bus tests are skipped on macOS.
See [rendering evidence](evidence/table-header-rendering-och41.md), including
clipped input restoration and resource teardown. A follow-up fixes clipping of
individual controls inside partially visible headers and prevents child-button
drags from arming column reorder. Native tests also cover Enter activation, bare
header drag and repeated-label group levels; physical desktop validation remains
separate.
Checked header/body-row styling now integrates through Core/Bonsai, Op114/115
and native state refinements. The gallery adds a public toggle. Targeted
Core/Bonsai/gallery, protocol **371** and native **815** (two existing private-bus
skips), thirteen table admission tests, strict lint and full OCaml build/tests/
formatting and a fresh independent installed-gallery build pass.
See [presentation evidence](evidence/table-scoped-presentation-och41.md) and the
[renderer-slot contract](design/table-renderer-slots.md). Physical macOS/catalog/
release gates remain open.

The message gallery now demonstrates native animated **Follow latest** controls
and optional bottom fading through public View/Animation composition. Visibility,
fade and motion switches preserve list geometry/anchors; hidden controls become
inert and settle outside the clip so scrolling remains usable. Full native **812**
(two existing private-D-Bus skips), full OCaml/gallery, strict lint/formatting and
a fresh installed-gallery build pass. Native tests cover exit input, reduced
motion, idle settlement and teardown. Physical scrolling/VoiceOver/GPU and broader
catalog/release gates remain open. See [message-follow evidence](evidence/message-follow-presentation-och41.md).

Structural tables now have a public `Table_view` builder with checked keyed
cells/rows/sections, grouped and row headers, actual column spans, captions and
embedded native controls. The Collections gallery demonstrates reversal without
recreating controls. Full OCaml/gallery, protocol **366**, native **811** (two
existing private-D-Bus skips), accessibility admission **4**, a real AppKit
no-window fixture, strict lint/formatting and a fresh installed-gallery build pass.
The physical gallery walkthrough is added but unrun; VoiceOver, remaining catalog
and release gates remain open. See [structural-table evidence](evidence/structural-tables-och41.md).

Managed tables now expose stable horizontal column observations through Core's
`Table.Column_viewport` and Bonsai `Output.column_viewport`, with a Result table
gallery readout. Measured pinned/scrolling panes handle empty data, single/all-pinned
schemas and partial clipping; stale callbacks are fenced and unchanged redraws
are silent. A misplaced native measurement canvas was corrected. Local validation
passes: protocol **365**, table adapter **3**, native **810** (two existing
private-D-Bus skips), admission **10**, full OCaml/gallery, strict lint, formatting
and a fresh independent installed-gallery build. Physical macOS and remaining
catalog/release gates remain open; see
[column observation evidence](evidence/table-column-viewport-och41.md).

Managed tables now expose native stripes, thirteen theme-resolved part colors and
shared/per-column padding through `Table.Appearance` and the Result table gallery.
Paint updates preserve geometry; padding updates fence stale input while retaining
selection, scroll owners and Bonsai cells. A short-table filler-row crash was found
and fixed. Full protocol **364**, native **808** (two existing private-D-Bus skips),
table admission **9**, full OCaml/gallery, strict lint, formatting and a fresh
independent installed-gallery build pass. The new
GPU readback case has not run; physical rendering/input and remaining catalog/release
gates stay open. See [presentation evidence](evidence/table-appearance-och41.md).

Managed tables now expose independent row-header visibility, Stop/Wrap navigation
and per-column-header selection through Core/Bonsai configuration and the Result
table gallery. Updates fence stale input, repair forbidden selections and retain
native owners/cell models. Paired protocol, atomic admission, native keyboard/AX/
layout and Bonsai tests pass: full protocol **363**, native **807** (two existing
private-D-Bus skips), table admission **8**, full OCaml/gallery, strict lint and
formatting. A fresh independent installed-gallery build also passes after the
header-navigation fix. Physical macOS acceptance, richer table presentation, structural table
semantics and the wider catalog/release gates remain open. See
[table behavior evidence](evidence/table-behavior-och41.md).

The selectable-list stack now includes `Gpuio_eio.List_search` and a public
Collections → **Searchable list** gallery. Scoped debounce, fetched-row merges,
hidden membership, cancellation, retry, source fencing and streamed-value conflict
handling have nine new Eio/Bonsai tests, including saturated-queue cleanup. The gallery demonstrates 1,000 entries,
sections, disabled options, both axes and bounded rows. Full OCaml tests/gallery
and formatting, structural catalog audit and a fresh independent installed gallery
build pass. A physical macOS walkthrough is added but has not run; native
input, VoiceOver, resource/performance and broader catalog/release acceptance remain
open. See [search/gallery evidence](evidence/selectable-list-search-och41.md) and
[managed component evidence](evidence/selectable-list-managed-och41.md).

## Previous implementation checkpoints

These record earlier stages; their remaining-work lists describe those checkpoints.
The current summary above takes precedence for selectable-list input.

The public Core selectable-list bridge now exposes typed `List_input` requests,
checked configuration and `View.with_list_input`, also available in the Bonsai View
facade. Query references use sibling View keys; reconciliation validates ownership,
assigns monotonic input generations and fences stale callbacks. Cursor/busy echoes
preserve ordered navigation, and clear/reinstall preserves the generation watermark.
Seven new Core tests, a Bonsai driver test, six exact public transactions replayed
by Rust, full OCaml tests/gallery, formatting/lint and a fresh installed-gallery/API
consumer pass. Managed selection/controller/search composition and its public gallery
remain unfinished; broader catalog and macOS release gates remain open. See
[Core input evidence](evidence/selectable-list-core-input-och41.md).

Production selectable-list input now handles native root/query focus, ordered
keyboard and matched pointer requests, independent selection/context, accessibility
actions and bounded cursor reveal. GPUI accessibility focus works with the query
on either side of the results; macOS desired-selection setters preserve every
queued value. Full native **805 passed/two existing private-D-Bus skips**, focused
admission/tree **43 passed**, both AppKit no-window fixtures, strict lint, full
OCaml tests/gallery/format and a fresh installed-gallery consumer pass. Public callbacks, Bonsai/Eio search, gallery and
physical qualification remain unfinished. See [native input evidence](evidence/selectable-list-native-input-och41.md).

Selectable lists now have paired Op110/Event74, checked ListBox/Option metadata,
native interaction-generation admission and atomic query ownership. Cursor/busy
updates preserve ordered navigation; query/editor changes validate affected owners
without expanding logical rows. Full protocol **362 passed**, native library
**797 passed/two existing private-D-Bus skips**, focused admission **35 passed**,
eight tree regressions, full OCaml/gallery, strict lint and formatting pass.
Native input listeners, AX actions, managed search and gallery integration remain
unfinished. See [bridge evidence](evidence/selectable-list-bridge-och41.md).

The standalone list foundation now includes incarnation-safe collection references
and a pure `List_selection` model: separate cursor/selection/context, checked
query visibility, disabled-aware navigation, ranges and confirmation. Core checks
cover all 256 eligibility masks and 100k loaded/selected metadata. Native input
listeners/AX actions, Bonsai/Eio search and the public gallery remain to implement;
this does not close the catalog gap. See [model evidence](evidence/selectable-list-model-och41.md)
and [contract](design/selectable-lists.md).

Horizontal managed lists now have public Core configuration, paired Op109,
production Host integration and reactive Bonsai constructors. The same bounded
row lifecycle, logical anchors, paging and tail state work along either axis;
changing configuration retains surviving row models and invalidates stale geometry
before paging. Collections adds 10k variable-width cards with growth/reorder,
commands, shared scrollbars and axis switching. Full protocol **359 passed**,
native **796 passed/two existing private-D-Bus skips**, targeted admission **19
passed**, full OCaml tests/gallery, strict lint, formatting, structural catalog
audit and a fresh installed gallery consumer pass. No OS windows opened; physical
macOS acceptance, remaining catalog gaps and the full release gates remain open.
See [integration evidence](evidence/horizontal-list-integration-och41.md),
[native engine evidence](evidence/horizontal-list-engine-och41.md) and
[implementation contract](design/horizontal-managed-lists.md).

Custom scrollbar presentation now reaches managed tables through a scoped native
adapter, preserving headers, pinned columns, selection and existing scroll
handles. Corner reservation respects actual sibling overflow; empty/all-pinned
cases and tree focus routing pass. A bound-action Escape regression is fixed in
the existing window key interceptor. Full native **791 passed/two existing
private-D-Bus skips**, admission, strict lint and format pass. Collections adds
shared mode/axis/style/motion/reset controls and a two-axis growth preview; gallery
link, full OCaml tests/format and a fresh installed-consumer build pass. Physical
macOS qualification remains unfinished.
See [table evidence](evidence/scrollbar-table-och41.md).

Core/Bonsai `View.with_scrollbar` now connects checked Op108 metadata and quota
admission to ordinary-container and managed-list native owners. Updates/reset
preserve existing handles and offsets; native range focus participates in Host
traversal. Full protocol **358 passed**, native **787 passed/two existing
private-D-Bus skips**, admission, strict lint, full OCaml tests/format and gallery
link pass. Host-wide drag cancellation, managed-table rendering, tree-specific
input coverage, public gallery controls,
installed-consumer and physical macOS qualification remain unfinished. See
[bridge evidence](evidence/scrollbar-bridge-och41.md).

The standalone shared scrollbar widget now paints real native ranges over existing
ScrollHandle/ListState owners, with captured drag, keyboard/AX routes, stable hover,
resize handling and lifecycle cleanup. Full native **785 passed/two existing
private-D-Bus skips**, strict lint/format and the gallery link pass; eight TestPlatform widget tests
exercise native behavior without OS windows. View/Host attachment was unfinished
at that earlier checkpoint. See [widget evidence](evidence/scrollbar-widget-och41.md).

Shared scrollbar timing now has paint-committed transitions and a GPUI owner
with cancellable idle deadlines and bounded weak frame callbacks. Eight lifecycle
and five native scheduler tests pass within the full **773-test native suite**
(two existing private-D-Bus skips); strict lint passes. The actual scrollbar
element and native input/accessibility were unfinished at that earlier checkpoint;
View bridge and gallery controls were unfinished then. See
[runtime evidence](evidence/scrollbar-runtime-och41.md).

The shared-scrollbar foundation now has checked Core values, independent paired
codec fixtures, native geometry and state-style resolution. Full protocol
**357 passed** and native **760 passed/two existing private-D-Bus skips**, plus
full OCaml tests/format/gallery build and strict Rust lint, pass.
Renderer/input/accessibility and runtime ownership were unfinished at that
foundation checkpoint. View bridge and gallery integration remain unfinished;
this is not yet a rendered public scrollbar capability. See
[foundation evidence](evidence/scrollbar-foundation-och41.md).

The pinned collection review now maps managed lists, scroll helpers, trees and
managed/structural tables, with remaining public gaps explicit. The Collections
gallery demonstrates bounded history insertion, append, latest-response growth
and native tail following. Public `Accessibility.Role.Log` adds transcript
semantics with explicit live policy; streaming previews use Off. Full protocol,
751 native library tests (two existing private-bus skips), seven admission/tree
checks, strict lint, full OCaml tests/format/gallery and a fresh installed-gallery
consumer build pass. Physical macOS and release qualification remain open; see [collection evidence](evidence/collections-catalog-och41.md)
and [source review](catalog/collections-review.md).

`Toast.Stack.Motion` now connects checked Core/Op107 configuration to production
native reflow, finite entry/exit and the Feedback gallery. Entry precedes active
expiry; dismissal immediately retires subtree input/restores focus and publishes
once after exit. Native composition and nested-modal overflow keep their precedence;
metadata updates retain editors and never replay entry. Full native **750 passed/
two existing private-D-Bus skips**, protocol **353 passed/no skips**, **23 focused
native tests**, strict lint, full OCaml tests/format/gallery and a fresh installed-
gallery consumer build pass. Physical macOS and release qualification
remain open; see [motion evidence](evidence/toast-motion-och41.md).

## Earlier checkpoints

The measured toast widget now supports adaptive native reflow and lifecycle paint
hooks. Streaming updates preserve width progress and the bottom anchor; interrupted
entry/exit uses actual painted samples, with inert exiting cards and no frame demand
from hidden/rejected previews. Accepted removal releases geometry before another draw.
Full native **741 passed/two existing private-D-Bus skips** and strict lint pass.
Public Motion, production phase timers and delayed dismissal remain unfinished;
the production Host still uses immediate presentation. See
[renderer evidence](evidence/toast-motion-renderer-och41.md).

`Toast.Stack.Layering` now connects current-frame measured layered cards through
Core/protocol admission to the production native Host and Feedback gallery.
Hover or named-scope focus expands; back cards are decorative and input-gated;
wheel/keyboard scrolling is bounded; metadata updates retain child editors.
Full native **735 passed/two existing private-D-Bus skips**, protocol **352
passed/no skips**, sixteen focused admission/lifecycle tests and Core/gallery
checks, strict lint/format and a fresh installed-gallery consumer pass. Coordinated native
entry/exit/reflow remains unfinished; its pure lifecycle/spring/token foundations
are separate from shipped behavior. See [layering evidence](evidence/toast-layering-och41.md)
and [motion foundation](evidence/toast-presentation-foundation-och41.md).

Toast stacks now support all eight edge/corner anchors and checked window margins
through `Toast.Placement`. Placement updates retain child editors and timeout state;
unusable geometry gates input/accessibility and pauses native expiry. Feedback adds
placement and reserved-margin controls. Full native **729 passed/two existing
private-D-Bus skips**, protocol **351 passed/no skips**, quota admission, Core
expect tests, strict lint, full OCaml tests/format/gallery and a fresh installed
gallery consumer build pass. Layered expansion and coordinated toast motion remain
required, alongside physical macOS and release qualification. See
[placement evidence](evidence/toast-placement-och41.md).

Core/Bonsai `View.split_group` now connects checked native admission, retained
panel/editor owners and asynchronous final observations to a public Navigation
gallery example. Reorder/hide/insert/remove, full bounds, reset/resize requests,
per-handle appearance and passive grips are implemented. Three production Host
tests, eleven admission/widget tests, full native **728 passed/two existing
private-D-Bus skips**, protocol **350 passed/no skips**, strict lint and full
OCaml tests/format/gallery and a fresh installed-gallery consumer build pass.
Physical macOS, catalog and release qualification remain open.
See [bridge/gallery evidence](evidence/split-group-bridge-och41.md).

The pinned notification review now maps existing toast behavior and separate OS
delivery against exact source snapshots. Layered expansion and coordinated
enter/exit/reflow remain required presentation work;
they are not claimed by the existing timeout/dismissal implementation. See the
[review](catalog/notification-review.md).

Earlier split-group checkpoint: a standalone measured native widget and checked
Core/native handle appearance. Nineteen targeted native tests, three paired codec
tests and the Core view API expect suite pass, including captured dragging,
keyboard/AX, clipping, custom grips and lifecycle cleanup. At that checkpoint the
tree/Host/event bridge, public view and gallery were unfinished. These checks
use TestPlatform; they do not establish physical desktop acceptance. See
[native widget evidence](evidence/split-group-widget-och41.md).

Earlier split-group foundation: a checked Core data model, independent paired
codec fixtures, constraint solver and retained geometry/request/drag state.
Eleven targeted Rust tests, including 2,000 generated groups, Core expect tests,
formatting and strict lint passed. The flat-group view, bridge, native handles and
gallery were unimplemented then; see the current bridge evidence above. See
[foundation evidence](evidence/split-group-foundation-och41.md).

Core/Bonsai tabs now accept optional `Tab_bar.Motion` for native indicator springs
and Pill selected-foreground fading. Interruption preserves painted velocity;
scrolling stays independent; reduced/hidden/inactive/removed owners stop motion.
The Navigation gallery adds **Animate tab selection**. Full native **725 passed,
two existing private-D-Bus skips**, protocol **346 passed/no skips**, fifteen
admission tests, strict lint, full OCaml tests/format/gallery and a fresh installed-
gallery consumer build pass. Flat split groups and physical/release qualification
remain required. See [motion evidence](evidence/tab-motion-och41.md).

All-tabs menus now support explicit Choice-ID keyed decorative SVGs through
`Tab_bar.Menu.create ~icons`. Closed/offscreen rows retain asset readers without
measuring icon rasters; visible rows render fresh native elements. Reorder,
source/style changes, reset, menu removal and window close have public fixture
and native resource/pixel tests. The Navigation gallery adds **Show tab menu
icons**. Full native **715 passed/two existing private-D-Bus skips**, fourteen
tab/menu admission tests, strict lint, OCaml tests/format and a fresh installed-
gallery consumer build pass. Tab indicator/color motion, flat split groups and
physical/release qualification remain required. See [icon evidence](evidence/tab-menu-icons-och41.md).

Core/Bonsai `Tab_bar.Menu` and `View.tab_bar_frame ~menu` now expose an all-tabs
menu with full names, selection checks, disabled options, native keyboard/
typeahead and menu accessibility. Selection leaves the tab viewport unchanged;
insertion/removal preserves its native owners. Full native **713 passed/two
existing private-D-Bus skips**, protocol **345 passed/no skips**, twelve tab/menu
admission/replay tests, strict lint, OCaml tests/format and a fresh installed-gallery
consumer build pass. Decorative menu icons, tab indicator/color motion, flat split
groups and physical/release qualification remain required. See [menu evidence](evidence/tab-menu-och41.md).

Core/Bonsai `View.tab_bar_frame` now supports fixed prefix/suffix controls and
ordinary trailing content inside the native viewport. Native focus reveals the
whole control border; hiding and reopening retains the previous scroll offset.
Full native **712 passed/two existing private-D-Bus skips**, protocol **344
passed/no skips**, ten admission/replay tests, strict lint, OCaml tests/format,
and a fresh installed-gallery consumer build pass. All-tabs menus,
indicator/color motion, flat split groups and physical/release qualification
remain open. See [frame evidence](evidence/tab-frame-och41.md).

Core/Bonsai tab constructors now accept a native horizontal `Tab_bar.Viewport`
and serialled stable-ID `Reveal_request`. Selection alone preserves the user's
offset; keyboard/assistive navigation and child focus reveal measured targets.
The public gallery contrasts Select last and Reveal last. Full native **711
passed/two existing private-D-Bus skips**, protocol **343 passed/no skips**, eight
admission/replay tests, strict lint, full OCaml tests/format/gallery and a fresh
installed-gallery consumer build pass. All-tab menus, bar
relationships, indicator/color motion and physical/release qualification remain
open. See [evidence](evidence/tab-viewport-och41.md) and
[design](design/rich-tabs.md).

Core/Bonsai structured tabs now support independent prefix/suffix controls,
decorative/default/hidden labels and maximum whole-tab width through
`View.Tab_content`, `View.tab_bar_with_content` and Op99. Close actions and child
editor input stay separate from tab selection; reorder preserves native owners.
The Navigation gallery adds close/reorder/restore and long-name truncation.
Exact public OCaml transactions replay in the native tree, including reset and
disposal. Full native **708 passed/two existing private-D-Bus skips**, protocol
**342 passed/no skips**, seven admission/replay tests, strict lint, full OCaml
tests/format/gallery and a fresh installed gallery build pass. Physical desktop
qualification, overflow/reveal, indicator/color motion and broader milestone 07
release gates remain open. See [evidence](evidence/tab-content-och41.md) and
[design](design/rich-tabs.md).

Core/Bonsai tab bars now support five static native variants and checked shared/
Choice-ID target styles through `Tab_bar.Appearance` and Op98. Appearance updates
preserve active keyboard choice, selection, native focus and rich label owners.
Plain/rich host tests cover target geometry, selected/hover/focused/disabled paint,
reorder, reset, idle frames and cleanup. The gallery cycles variants and custom
sizes. Full native **707 passed/two existing private-D-Bus skips**, protocol **341
passed/no skips**, five admission tests, strict lint, full OCaml tests/format/gallery
and a fresh installed gallery build pass. Interactive parts, max-label-width,
overflow/reveal, indicator motion and physical/release acceptance remain open.
See [evidence](evidence/tab-appearance-och41.md) and [design](design/rich-tabs.md).

Core/Bonsai tab bars now accept checked decorative labels keyed by Choice ID.
Native admission enforces bounded slots and late descendant checks; labels paint
once while native tab names, keyboard behavior, disabled state and owner identity
remain intact. The Navigation gallery toggles badges without replacing editor
panels. Full native **706 passed/two existing private-D-Bus skips**, four label
admission tests, strict lint, full OCaml checks and a fresh installed gallery build
pass. Full rich-tab variants/interactive parts/overflow/motion and physical
qualification remain open. See [evidence](evidence/tab-labels-och41.md) and the
[source-reviewed design](design/rich-tabs.md).

The public Journeys gallery now demonstrates measured carousels with unequal
retained cards, a persistent editor, both axes, reordering, viewport resizing,
looping, motion and automatic advancement. A Bonsai window-driver regression
checks ordered requests, stale automatic/layout events, retained IDs, disabled
input and idle/close behavior. Full OCaml tests/format/gallery build and a fresh
installed-package gallery build pass. The physical walkthrough is documented but
unrun; milestone 07 remains open. See [gallery evidence](evidence/carousel-track-gallery-och41.md).

Measured carousel cards now retain layout/native owners while fully clipped
children are excluded from input, Tab and accessibility. Focus returns to the
eligible viewport when its child leaves view; nested-scroll reveal, stable IDs,
reorder/resize, modal independence and anchored-popover suspension/resumption
have native host regressions. A visibility transition requests one bounded redraw
so retained popovers reappear without unrelated input. Full native **705 passed/
two existing private-D-Bus skips**, five transport tests, strict Rust lint and
full OCaml tests/format/gallery build pass. Physical AX/IME and gallery/public-driver
qualification remain open. See [visibility evidence](evidence/carousel-track-visibility-och41.md).

Measured carousel tracks now have an explicit checked group connecting their
viewport and default controls. Pointer activation focuses the viewport;
keyboard/assistive activation preserves control focus, and related controls route
axis/Home/End keys without taking keys from card editors. Group hover and control
focus pause automatic advancement; cards expose Group position/count metadata.
Admission checks include child-only and nonstructural updates. Full native **700
passed/two existing private-D-Bus skips**, protocol **339 passed/no skips**, five
transport tests, strict lint and full OCaml tests/format/gallery build pass. A fresh
installed gallery and independent Core/Bonsai group probe also pass. Offscreen
focus/reveal, full AX qualification, public gallery/driver coverage and physical
release acceptance remain open. See [control evidence](evidence/carousel-track-focus-och41.md).

Measured carousel tracks now support precise-trackpad pixel previews, nearest-item
release and one measured navigation step per line-wheel burst. One cancellable
quiet deadline handles missing terminal events, with source/epoch validation and
momentum fencing across controlled-model echoes. Native child scrollers keep first
refusal; vertical edge gestures can reach enclosing scrollers while horizontal
tracks retain their axis. Cancellation covers Escape, capture, geometry/policy/source
changes and disposal. Full native **698 passed/two existing private-D-Bus skips**,
four transport tests, strict lint and full OCaml tests/format/gallery build pass.
Full focus/AX/control relationships, gallery/public-driver integration and physical
macOS/release acceptance remain open. See [wheel evidence](evidence/carousel-track-wheel-och41.md).

Measured carousel tracks now support captured background dragging in actual track
pixels, with axis lock, finite bounds, continuous rebasing and nearest-item release.
The native host keeps controlled selection unchanged until OCaml accepts a request.
Both-axis tests verify out-of-bounds dragging and no per-frame bridge events;
additional tests cover native button/editor precedence, text selection/typing,
Escape, foreign capture, changed geometry/policy/source and unmount/window-close
cleanup. Frame handlers share admitted configs. Full native **690 passed/two
existing private-D-Bus skips**, four transport tests, strict lint and full OCaml
tests/format/gallery build pass on the final implementation. **Full focus/AX/control relationships and gallery/public-driver coverage
remain unfinished; wheel handling is covered by the newer checkpoint above.** No physical macOS or Linux desktop qualification is claimed.
See [pointer evidence](evidence/carousel-track-pointer-och41.md),
[native keyboard/deadline evidence](evidence/carousel-track-navigation-och41.md),
[motion evidence](evidence/carousel-track-motion-och41.md) and
[renderer evidence](evidence/carousel-track-renderer-och41.md). The motion checkpoint
also records 339 passing protocol tests and fresh installed Core/Bonsai API probes.

Earlier carousel checkpoints establish the
[geometry foundation](evidence/carousel-track-foundation-och41.md),
[Core model and payloads](evidence/carousel-track-model-och41.md), and
[checked bridge/publication state](evidence/carousel-track-bridge-och41.md).
Those include the paired protocol's **338 passing tests**, Core/Bonsai construction,
stale-source rejection and installed-package probes. They remain supporting
evidence; neither those checkpoints nor measured rendering close OCH-41/OCH-17.

Sidebar destinations now support independent per-item and label styles through
retained composed links, preserving 4096-byte meaningful Unicode names. Blank
names are explicitly rejected at construction. Native tests found and fixed link
child disabled/semantic projection and excessive debug stack use during nested
rendering. The Journeys styling toggle, three Core expect tests, exact public
transaction replay and simulated native pointer/keyboard/AX checks pass. Full
native **661 passed/two existing private-D-Bus skips**, strict Rust lint, full
OCaml tests/format/gallery and a fresh installed gallery consumer build pass.
Physical macOS and broader catalog/release qualification remain open. See the
[contract](design/sidebar-styling.md) and [evidence](evidence/sidebar-styling-och41.md).

Seventeen pinned sidebar/navigation/carousel/tab/resizable source snapshots now
have a [feature-level review](catalog/journey-workspace-review.md). Sidebar items
support opt-in select-only/expand/toggle activation, preserving independent caret
behavior and latest-model eligibility. Journeys exposes the policy on the retained
sidebar. Two new expect tests, full OCaml tests/format/gallery build and a fresh
installed gallery consumer pass. The review explicitly retains measured carousel-track, rich/overflow tab and
split-group/handle work within
OCH-41; these are not silently deferred with comprehensive docking. Physical and
broader release acceptance remain open. See [evidence](evidence/sidebar-activation-och41.md).

Calendar viewport observations now independently report native day/month/year panes
through `Calendar.Viewport`, opt-in Core/Eio callbacks and Op95. The gallery loads
sparse date/header content from exact grid dates; selection revisions and pending
picker confirmation remain intact. Tests also found and fixed generic native input
and overload publication after terminal shutdown. Full native **659 passed/two
existing private-D-Bus skips**, protocol **333 passed/no skips**, full OCaml tests/
format/gallery build, strict Rust lint and a fresh installed gallery consumer build
pass. Physical macOS and broader catalog/release acceptance remain open. See the
[contract](design/calendar-viewport.md) and [evidence](evidence/calendar-viewport-och41.md).

Sheets now accept checked application content insets through `Sheet.Insets`,
`Sheet.Config` and Op94. All-edge layout responds to resize while retaining the
full-window backdrop, focus contract, child owners and entry clock. Native tests
found and fixed padding/border overflow in tiny sheets, including combined hover/
focus styles. The gallery exposes reserved chrome space and all four edges.
Full native **656 passed/two existing private-D-Bus skips**, protocol **331 passed/
no skips**, strict Rust lint, full OCaml tests/format/gallery build and a fresh
installed gallery consumer build pass. The pinned custom-window/title-bar source
mapping is explicit; no inferred OS safe-area guarantee is added. Physical macOS,
remaining catalog/release validation and milestone 07 acceptance remain open.
See the [contract](design/sheet-insets.md) and [evidence](evidence/sheet-insets-och41.md).

Popup placement now supports four window-point corners and bounded viewport
margins through `Placement.at_point`, `Placement.create` and Op93. Native current-
frame placement clamps points without flipping, preserves popup owners/focus,
and keeps nested menus attached to their own rows. Existing absolute dialog
insets are also verified. The public gallery includes live placement controls.
Four Core tests, paired codec/admission checks and five native host regressions
pass; full native **651 passed/two existing private-D-Bus skips**, protocol
**330 passed/no skips**, strict Rust lint, full OCaml tests/format/gallery build
and a fresh installed gallery consumer build pass. Physical macOS, remaining
sheet inset review and milestone 07 acceptance remain open. See the
[contract](design/placement-geometry.md) and [evidence](evidence/placement-geometry-och41.md).

Tooltips now support opt-in `Tooltip.Motion.Enter_and_switch` through Config and
Op92: native entry, same-row slide, cross-row immediate placement and ordered
managed replacement. Controlled tips still require accepted application updates.
Only visible panel paint supplies history; hidden peers retain accepted state,
replacement batches synchronization, and reduced motion/cleanup stop frames.
The full native suite passes **644 tests/two existing private-D-Bus skips**;
the final five focused host tests also pass, including the subsequently added
hidden-peer/batch regression. Protocol **329 passed/no skips**, strict Rust lint,
full OCaml tests/format/gallery build and a fresh installed gallery consumer build
pass. Physical qualification and milestone 07 remain open. See the
[contract](design/tooltip-motion.md) and [evidence](evidence/tooltip-motion-och41.md).

Modal helpers now expose opt-in `Overlay.Motion.Enter` through Core/Bonsai and
Op91. Native dialog/alert fade-and-slide and all-edge sheet entry run inside the
deferred surface, with stable geometry/AX identity, reduced motion, no idle timer
and immediate removal. Tests found and fixed missing eventual autofocus when a
top sheet's controls begin offscreen: the modal scope holds focus through entry,
then resolves a child unless the user already chose one. Full native **638
passed/two existing private-D-Bus skips**, protocol **328 passed/no skips**,
strict Rust lint and full OCaml tests/format/gallery build pass. A final focused
hide/show regression also passes. A fresh installed gallery consumer builds
successfully. Physical macOS and milestone 07 acceptance remain open. See the
[contract](design/overlay-motion.md) and [evidence](evidence/overlay-motion-och41.md).

The pinned overlay/help review now covers 22 exact source snapshots and maps
functional equivalents and remaining presentation gaps. Core/Bonsai modal helpers
accept theme-aware backdrop colors. A native regression found and fixed missing
modal accessibility flags caused by type erasure. All six modal kinds retain
focus and input blocking through tinted/transparent/default paint changes;
popovers remain nonmodal. Full OCaml tests/format/gallery build, native **634
passed/two existing private-D-Bus skips**, protocol **327 passed/no skips** and
strict Rust lint pass. A fresh installed gallery consumer builds successfully.
Physical qualification and the new desktop walkthrough
remain open. See the [review](catalog/overlay-review.md),
[contract](design/overlay-backdrop.md) and [evidence](evidence/overlay-backdrop-och41.md).

Calendar content now supports checked passive day, navigation and heading slots
through `View.Calendar_content`, native Op89 admission/rendering and managed picker
helpers. The gallery shows live event badges. Tests caught and fixed duplicate
rendering through generic child layout. Native focus/range/AX identity,
read-only selection, hidden animation suspension and cleanup pass; a driver test
preserves pending confirmation during content updates. Full OCaml tests/format/
gallery build pass; native **633 passed/two existing private-D-Bus skips**, protocol
**326 passed/no skips**. Strict Rust lint and a fresh installed gallery consumer
build pass. The authored desktop walkthrough is unrun. Physical
qualification and exact viewport observation remain open. See the
[contract](design/calendar-content.md) and [evidence](evidence/calendar-content-och41.md).

Pagination now gives each gap its own persistent native popup anchor through
`Navigation.Gap_popup`. Placement, independent expanded state and focus return
follow the selected gap; the managed chooser retains one bounded numeric owner.
Direct-button popovers now return focus to their eligible declared anchor even
after semantic activation or replacement of another popup, with previous-focus
fallback and no stealing focus from outside. Exact public transaction replay,
Core/driver tests and native policy/lifetime regressions pass. Full native checks
pass **631 tests/two existing private-D-Bus skips**; strict Rust lint and full
OCaml tests/format/gallery build pass. The updated physical gallery walkthrough
is authored but unrun. Milestone 07 and physical macOS qualification remain open.
See [contract](design/pagination-chooser.md) and
[evidence](evidence/pagination-chooser-och41.md#per-gap-popup-ownership-follow-up).

Shared `View.popover` now supplies expanded and dialog-popup accessibility state
on direct button/command-button anchors, including plain/rich date/color pickers.
It preserves semantic identity and existing activation/focus/dismissal ownership;
state follows the accepted native surface. Four native tests cover mouse/keyboard/
AX routing, nested/custom anchors, disabled/inert/pointer gates and cleanup. A
stale AX Click after disabling a button is now consumed instead of becoming a
pointer click. Core fixtures, native admission and public picker drivers pass.
Full native **630 passed/two existing private-D-Bus skips**, protocol **323 passed**,
strict Rust lint and full OCaml tests/format/gallery build pass. Physical desktop
qualification remains open; pagination gap composition is covered by the newer
checkpoint above. See
[contract](design/picker-triggers.md#shared-popover-trigger-state) and
[evidence](evidence/picker-triggers-och41.md#shared-popover-accessibility-state).

Color input Palette/HSLA tabs now retain one native model and five text editors.
`Color_input.Panels` supplies localized labels and initial selection; appearance
updates retain the user's selected tab. Hidden channel text settles by blur
policy, capture cancels, visible hex composition survives presentation updates,
and eligible Focus commands reveal Channels. Native mouse/keyboard/AX, modal and
policy gates, stale callbacks and disposal pass five new TestPlatform tests.
The gallery uses tabs in inline and popup pickers. Full native checks pass
**626 tests/two existing private-D-Bus skips** and full protocol **322 tests**;
full OCaml tests/format/gallery build and strict Rust lint pass. Physical macOS and Linux desktop
acceptance remain open. See the [contract](design/color-presentation.md) and
[evidence](evidence/color-presentation-och41.md).

Color inputs now support labeled grouped/featured palettes and bounded,
theme-aware swatch/channel geometry through `Palette_section`,
`Config.palette_sections` and `Color_input.Appearance`/Op87. The gallery shows
favorites and nine color families, plus a compact scrollable popup. Group/style
updates retain the native owner, editors, composition and focus handles; section
counts and retained metadata are validated atomically. Three Core tests, one
popup-driver test, native layout/ownership and codec/admission checks pass.
Full native **621 passed/two existing skips**, protocol **321 passed**, strict
Rust lint and full OCaml tests/format/gallery build pass. Physical macOS validation
remains open; the tab continuation is recorded above. See the
[contract](design/color-presentation.md) and
[evidence](evidence/color-presentation-och41.md).

Color palette hover now previews the swatch and hex spelling without changing
the native color, editor draft, composition, selection, history or focus. The
caption reserves its space to avoid layout jumps. Native hit testing, duplicate
slots, stale callbacks, pointer-only policy changes during composition, capture,
modal gates and cleanup are covered by three new TestPlatform tests. **620 native
tests pass (two existing private-D-Bus skips)**, strict Rust lint passes, and full
OCaml tests/formatting/gallery build pass. No OS windows were opened; physical
macOS acceptance remains open. Grouped palettes, panel switching and internal
color styling remain OCH-41 work. See the [contract](design/color-palette-preview.md)
and [evidence](evidence/color-palette-preview-och41.md).

Date and color pickers now expose checked rich triggers with independent button
styling. The gallery displays formatted dates and color swatches with adjacent
clear actions that cancel open drafts before clearing application values. Three
new Bonsai-driver tests verify trigger/draft identity, stale actions, disabling
and invalid composition. The shared passive validator also rejects command-binding
observers before submission, with a Core regression assertion. Full OCaml tests,
formatting and the final gallery build pass. The updated desktop walkthrough is
syntax-checked but unrun; physical focus/AX/rendering acceptance remains open.
See the [contract](design/picker-triggers.md) and
[evidence](evidence/picker-triggers-och41.md).

Calendars now support 1..12 consecutive wrapping months, one shared native
selection/focus owner, and bounded theme-aware cell geometry through
`Calendar.Appearance`/Op86. Cursor-month snapshots remain unchanged; a separate
native display anchor avoids jumps between already-visible panes. The gallery
shows 1/2/3/12 months. Core fixture/ownership tests, native range/AX/layout/retention
checks and atomic admission pass. Full OCaml tests/format/gallery build, **617
native tests (two existing skips)**, **320 protocol tests**, and strict Rust lint
pass locally. Physical macOS/resources/consumer/release acceptance remains open.
See the [contract](design/calendar-presentation.md) and
[evidence](evidence/calendar-presentation-och41.md).

Date pickers now have bounded, typed presets that update the native draft and
retain explicit Apply. Latest configuration/session/revision checks and a pending
command gate prevent stale replies or overlapping confirmation from committing
unexpected values. The gallery includes date/week/clear shortcuts. Two new Core
and seven Bonsai-driver tests pass; full OCaml tests, formatting and gallery build
pass, followed by the final button-routing test. No OS windows or new native
protocol operations were involved. The pinned calendar/color source review now
records multi-month, internal appearance, grouped palette, panel switching and
hover-preview gaps explicitly. See the [contract](design/date-picker-presets.md),
[evidence](evidence/date-picker-presets-och41.md) and
[source review](catalog/calendar-color-review.md). Milestone 07 remains incomplete.

Core/Bonsai disclosure now includes bounded rich accordion titles, validated
heading levels, per-item styling and opt-in `Disclosure.Motion`. Measured reveal
is connected through Op85 to the native host. Immediate remains the API default;
the gallery offers an Animate toggle. Retained closing visuals are inert;
Unmount disposes descendants immediately and supports animated reopening.

Nine motion-state, five layout and two native host tests cover reversal, wrapping,
focus/draft retention, hidden AX ancestry, blocked outgoing input, reduced motion,
clipping, inactivity and cleanup. Three additional Core tests and paired codec/
admission checks cover the public API, stable identity and atomic style-only
visibility enforcement. The full native library suite passes **614 tests with
two existing skips**; all **319 protocol tests** and **22 existing tree/session/
disclosure/navigation/allocation regressions** pass. Full OCaml tests, formatting,
gallery build, strict Rust lint and catalog audit pass. Physical macOS/resources/
release acceptance remains open; see the [contract](design/disclosure-presentation.md),
[source review](catalog/disclosure-review.md) and
[evidence](evidence/disclosure-presentation-och41.md).

Navigation now includes compact paging, interactive gaps, passive intermediate
breadcrumbs and per-member styling. `Gpuio_eio.Pagination` supplies a bounded
page chooser with seven shortcuts and a native numeric field, fresh opening
identity and guarded asynchronous confirmation. The gallery demonstrates actual
route changes and large/empty/shrinking/disabled page models. Three added Core
tests, five Bonsai driver race tests and an exact transaction fixture pass. Native
TestPlatform verifies keyboard, focus and accessibility routing; the full native
suite passes **597 tests with two existing skips**. Full OCaml tests, formatting,
gallery build, strict Rust lint and catalog audit pass. Physical macOS, measured
resources and release acceptance remain open. See the
[contract](design/pagination-chooser.md), [source review](catalog/navigation-review.md)
and [evidence](evidence/pagination-chooser-och41.md).

Sliders now expose `Slider.Appearance` for selected/remaining fill, independent
track/thumb/focus colors and bounded geometry, plus native hover/pressed spring
rings. Visual changes preserve values/focus; target-size changes cancel capture.
Motion is native, settles without idle frames, follows reduced motion and retires
on hidden/inactive/policy/owner changes. Gallery controls demonstrate presentation.
**596 native library tests and six slider tests pass**, with two existing skips;
three final focused lifecycle tests, strict Rust lint, formatting, full OCaml tests,
gallery rebuild and catalog audit pass. The wire is unchanged from the preceding
317-test protocol checkpoint. Physical macOS input/visual/AX, measured resources
and release acceptance remain open. See the [contract](design/slider-presentation.md)
and [evidence](evidence/slider-presentation-och41.md).

Application-controlled numeric stepping now has opt-in `Config.step_mode`, guarded
request/resolution APIs and native key/button/AX delivery. Held buttons pause
without a repeat timer while waiting, then resume only for a valid gesture.
Typing, policy/visibility changes and deactivation invalidate pending replies;
mode-only updates preserve the editor. The gallery demonstrates step sizes chosen
in OCaml. **316 protocol tests, 592 native library tests and seven numeric
admission tests pass**, with two existing skips; strict Rust lint, formatting,
full OCaml tests, gallery build and catalog audit pass. Managed Eio handlers now cancel scoped computation, suppress late callbacks and
queue guarded cleanup declines even with all 64 ordinary command slots occupied.
Five Eio lifecycle tests and an App routing/capacity test pass; the page-scoped
gallery, full OCaml tests/formatting and catalog audit pass. Physical macOS
validation and release acceptance remain open. See the [request contract](design/number-step-requests.md).

Numeric inputs now expose `Number_input.Appearance` and `View.number_frame` for
integrated leading/trailing content, passive custom step symbols, part styling
and full-frame focus treatment. Native editing, composition, history and hold
repeat remain owned by the same editor. The gallery exposes these controls.
**314 protocol tests, 585 native library tests and six numeric admission tests
pass**, with two existing native skips; strict Rust lint, formatting, full OCaml
tests, gallery build and catalog audit pass. Application-controlled stepping and
physical macOS/release acceptance remain open. See the
[contract](design/number-presentation.md) and
[evidence](evidence/number-presentation-och41.md).

OTP caret blinking now uses one native cancellable timer while focused, visible
and eligible. Editing restarts the visible phase; appearance changes preserve it.
Composition/read-only/reduced-motion states stay steady. Tests cover paint-only
ticks, no bridge events, retained IME geometry, clipping, inactive/disabled/hidden
states and teardown. Window Close explicitly retires OTP timers even if a
platform handler still holds the editor. **584 native library tests pass with
two existing skips**, as do the extended ancestor-clip check, strict Rust lint,
formatting, full OCaml tests and gallery build. Physical macOS timing/input/
VoiceOver, measured resources and release acceptance remain open; see the
[contract](design/otp-caret.md) and [evidence](evidence/otp-caret-och41.md).

Segmented OTP inputs now expose `Otp_input.Appearance`: grouped cells, bounded
dimensions and theme-resolved colors on the same retained native editor. The
gallery changes grouping and cell size without replacing editing state. Paired
fixtures, atomic admission and TestPlatform checks cover paint, shared hit/caret
geometry, stale-layout invalidation, history and provisional composition.
**313 protocol tests, 581 native library tests and seven OTP admission tests
pass**, with two existing native skips. Strict Rust lint/formatting, full OCaml
tests/formatting, gallery build and catalog audit pass. Caret timing is recorded
above; physical macOS visual/input/VoiceOver acceptance remains open. See the
[contract](design/otp-presentation.md) and
[evidence](evidence/otp-presentation-och41.md).

Public `Gpuio.Stepper` now provides application-owned workflow navigation with
stable identities, current/completed/upcoming presentation, horizontal/vertical
layouts, custom passive content, localization and disabled-state guards. The
gallery keeps a notes editor alive across stage changes. Five Core/fixture tests
and the exact OCaml transaction rendered on TestPlatform cover reconciliation,
Tab/Enter/Space, native AX metadata and queued activation after disable. The
fixture exposed a default-debug-stack overflow; extracting nonrecursive native
event/focus decoration fixes it without increasing stack limits. Full native
**580 tests pass with two existing skips**, strict Rust lint, full OCaml tests,
formatting and gallery build pass. Physical macOS visual/keyboard/VoiceOver and
release/resource acceptance remain open; see the [contract](design/workflow-stepper.md)
and [evidence](evidence/workflow-stepper-och41.md).

The numeric gallery now demonstrates retained unfinished drafts, separate committed
values, all three step-control arrangements, commit/restore/history, optional-empty
and disabled policies. Full OCaml tests, formatting and gallery build pass. A new
[pinned numeric/OTP review](catalog/numeric-review.md) records presentation and
custom-step-policy gaps. It also corrects a catalog error: upstream
`component/stepper` is workflow-stage navigation, not numeric increment/decrement.
Its local implementation is recorded above; physical acceptance remains open. No new physical desktop acceptance is claimed.

Text-area search now includes public `Gpuio_eio.Search_bar` presentation and a
Find/Replace gallery example, alongside typed commands and automatic native
observations. Query/replacement composition, checked replacement, scoped shortcuts,
retained document identity and late replies have deterministic Bonsai coverage.
Closing checks the current opening and restores native document focus when
eligible. The native suite passes **579 tests with two existing skips**;
**311 protocol tests**, full OCaml tests, gallery build, formatting and strict Rust
lint pass. Eight search-bar driver tests and the Eio suite pass after the final
OCaml-only feedback/retention changes. Physical macOS keyboard/focus/IME,
accessibility, visual and resource acceptance remain required. See the
[contract](design/textarea-search.md),
[search-bar evidence](evidence/search-bar-och41.md),
[observation evidence](evidence/textarea-search-observations-och41.md) and
[command evidence](evidence/textarea-search-commands-och41.md).

Native editor viewport queries and scroll requests now have typed OCaml/Eio
APIs, paired command/reply fixtures, and a public gallery demonstration. Queries
return coherent last-layout metadata; scroll acknowledgements mean accepted,
not painted. A reproduced IME scroll snap is fixed by tracking the same logical
caret key across layout and paint, including masked Unicode. Actual TestPlatform
wheel dispatch verifies observations between input and layout. The full native
suite passes **563 tests with two existing skips**; full OCaml tests, formatting
and gallery build pass. All 309 protocol tests, strict Rust lint and Rust formatting
checks pass. Physical macOS acceptance remains open. See the [contract](design/editor-viewport.md) and
[evidence](evidence/editor-viewport-och41.md). Search/replace is still required.

Native inputs now expose opt-in `Text_input.Config ?clear_on_escape` for both
modes. Current native editability/focus and format/filter rules guard an undoable
clear; composition takes precedence and empty/read-only/disabled fields preserve
normal propagation. Picker queries reserve Escape for their own behavior. The
Editors gallery demonstrates toggling on retained drafts. Two Core expect tests,
**558 native library tests with two existing skips**, four editor/twelve picker
checks, 308 protocol tests and full OCaml tests/formatting/gallery pass. A
reproduced input-frame clear rejection now preserves directed selection and undo
history using the same guarded replacement path. Final strict Rust lint passes. Physical macOS acceptance remains open; see the
[contract](design/editor-escape.md) and [evidence](evidence/editor-escape-och41.md).

Ordinary multiline inputs now expose `Text_area_layout` for wrapping,
continuation indentation, whitespace indicators and bounded cursor margins.
Paired Op78 and retained native configuration preserve text, composition,
selection, focus and history. Geometry tests cover wrap/indent, horizontal scroll
survival and directional caret margins without redraw jitter. Two Core expect
tests and the independent protocol fixture pass; the full native library suite
passes **555 tests with two existing skips**, plus two new admission checks.
The Base adaptation reconstructs all 233 vendor files exactly. Full OCaml tests, formatting, gallery and strict Rust lint checks pass;
physical visual/input and release acceptance remain open. See the [contract](design/textarea-layout.md) and
[evidence](evidence/textarea-layout-och41.md). Ordinary search/replace remains required work; viewport APIs are recorded above.

Native regex edit filtering now connects `Input_validation.regex` and
`Text_input.Config ?edit_filter` to paired Op77, atomic native admission and one
retained compiled policy/cache. Formatting precedes filtering; incompatible drafts
and history survive policy changes, while exact replacements check the current
rule. Submissions still require application validation. The gallery now demonstrates
handle/reference/free modes with rules prepared once during Eio startup.

Five public-boundary expect tests, two cancellation scheduling tests, paired
fixtures, **552 native library tests with two existing skips**, two admission tests
and eleven picker checks pass. A rendered-host accessibility test also passes
queued replacement across filter change and policy/cache release on removal.
Full OCaml tests, formatting and the gallery build pass. These are local automated
checks, not physical macOS input/VoiceOver, measured performance or release
acceptance. See the [contract](design/input-validation.md) and
[evidence](evidence/input-validation-och41.md).

Rendered formatting tests compare caret geometry with a plain reference after
native editing and exercise queued accessibility replacements across policy change
and removal. The test-only GPUI accessibility helper patch reconstructs all 155
files exactly. See [formatting evidence](evidence/input-formatting-och41.md).

Ordinary single-line inputs now expose `Text_input.Config ?format` for bounded
pattern masks and precision-preserving grouped decimals. Paired operation 76,
atomic native admission and a Rust-only retained editing policy preserve owner,
draft, selection and history across policy changes. Explicit commands reject
noncanonical replacements; interactive edits map the caret through formatting.
Provisional IME text can differ from the format and commits or cancels as one
composition; enabling policy during an existing composition is covered by a
reproduced and repaired regression. The gallery Editors page now has reference,
decimal and free-text examples with guarded sample replacement.

The earlier format-only checkpoint passed **543 library tests, two existing
skips, eight Base formatting checks, two format-admission checks and ten picker
checks**. Nine Core formatting expect tests and paired operation fixtures pass;
all 304 protocol tests and strict Clippy pass. The vendor patch reconstructs
all 233 files exactly. The
full OCaml tests, formatting and gallery rebuild pass after updating both
independent backend lockfiles for the existing Unicode dependency. Physical macOS IME/clipboard/AX,
visual, resource and installed-consumer acceptance remain open. Native regex
filtering is recorded above; the remaining plain-input catalog is still required.
See the [design](design/input-formatting.md) and
[checkpoint evidence](evidence/input-formatting-och41.md).

Semantic input hints now include 45 typed values, paired protocol data, atomic
password-privacy checks, retained AX/focus routing and an asynchronous native
exposure-status query. The gallery demonstrates configuration and query-time
status without promising autofill. Independent codec fixtures, Eio request
lifecycle/controller tests, native TestPlatform focus checks and the earlier real
headless AppKit property fixture pass. Full OCaml tests, formatting and the gallery
build pass. Actual macOS desktop/autofill and final platform qualification remain
open; see the [contract](design/input-content-hints.md) and
[partial evidence](evidence/input-content-hints-och41.md).

`View.input_frame` now composes leading/trailing content, an undoable native clear,
loading/busy state and application-controlled password reveal around the retained
editor. Core/paired-codec/admission and TestPlatform checks pass. A reproduced
multiline frame-height defect is repaired; tests cover fixed height, auto-grow,
narrow layout, pointer clear, stale actions and composition. The full native
library suite passes 534 tests with two existing skips. Full OCaml tests,
formatting and gallery build also pass. Actual macOS visual/input/AX acceptance
remains open. See the [contract](design/input-frame.md)
and [evidence](evidence/input-frame-och41.md).

`View.editor_menu` now supplies an opt-in Cut/Copy/Paste/Select-all menu for a
single-line input or text area. It binds to the exact native editor and checks
identity, current policy and focus again after deferred delivery. Config changes
and keyed sibling reorders preserve the field. Core/codec/admission and native
TestPlatform checks pass; full OCaml tests/formatting/gallery build and 532 native
library tests pass (two existing skips). The password gallery demonstrates the
helper. Actual macOS input/clipboard/AX/visual acceptance remains open. See the
[contract](design/editor-menu.md) and [evidence](evidence/editor-menu-och41.md).

Password inputs now support application-controlled hide/reveal on a retained
native editor. Both modes suppress the accessibility value; hidden mode blocks
Copy/Cut. Core, paired codec, atomic admission and TestPlatform checks pass for
identity, selection, undo, marked composition, clipboard and AX-tree behavior.
The public gallery demonstrates reveal, read-only, disabled and submission using
synthetic text. The public gallery builds with this API; current full-suite
totals are recorded above. These are local build/TestPlatform results, not macOS
desktop acceptance. See [password evidence](evidence/editor-privacy-och41.md).

The plain-input catalog review records fourteen exact upstream snapshots.
Formatting/native validation and ordinary text-area search/layout controls
remain v1 implementation work; only
full code-editor/LSP subfamilies are deferred. See the [review](catalog/editor-review.md)
and [extension plan](design/plain-input-extensions.md).

Application teardown now completes remaining requests and releases other windows
even when one completion callback raises, then propagates the first failure with
its original backtrace. Five inline tests use real Eio fibers/native transport
allocation with injected host events; they cover closure, stale identities, the
shared request limit and exception-safe cleanup. Full OCaml tests, formatting and
the gallery build pass. This is not native event-loop or desktop shutdown
acceptance. See [command lifecycle evidence](evidence/command-lifecycle-och17.md).

The additive `Choice_picker` now connects validated Core types, paired codecs,
atomic native admission, retained Rust owners, grouped variable-height rows and
public Core/Bonsai/Eio APIs. It supports single/multiple selection, popup search,
rich rows and trigger content, interactive footer, custom empty content, clearing,
controlled/managed visibility and ordered query/selection/open observations.
Current-model and editor-generation checks reject stale requests; query state
survives closure. This is implemented functionality under validation, not completed
catalog or macOS release acceptance.

Native TestPlatform checks cover virtualization, clipping/controlled recovery,
Clear/Tab/Escape, nested ownership, named accessibility groups and query/option
focus during composition. Regressions repaired duplicate query/footer mounting,
controlled acceptance losing focus handoffs, stale offscreen text measurements
and long labels failing to wrap in narrow popups. Resize/scale checks retain the
logical row and pixel offset; a constrained query/footer popup remains bounded,
and settled draws schedule no frame callbacks. The latest full native suite passes
**530 library tests plus eight admission tests**, with two existing library skips.
See the [picker contract](design/choice-picker.md) for exact guarantees and limits.

Four direct Bonsai/Window_driver controller tests cover observation-before-effect
sampling, revision monotonicity, delayed replies and query retirement. Disabling
search now returns `Not_mounted` without sending conditional replacement commands.
These use controlled command completions, not native Eio I/O or shutdown coverage.
Full OCaml tests, formatting and gallery build pass at the latest OCaml checkpoint.

The public gallery includes grouped rich multiple selection, controlled single
selection, 4,096 searchable workspaces and an empty/create/reset workflow. Its
current-permission reducer and catalog-admission tests pass, as does a fresh
independently installed gallery/backend build. The catalog source audit passes.
The desktop walkthrough is authored and Python-compiled but unrun. The picker
GPUI adaptation has 19 passing isolated accessibility checks and exact archive
reconstruction; later focus/layout repairs do not change that fork.

Remaining picker acceptance includes physical macOS input/IME, external AX and
VoiceOver, visual gallery review, installed-consumer runtime, broader resource and
window-lifecycle workloads, and required Linux checks against the final revision.
The black-window startup issue is unresolved. No TestPlatform/build result claims
desktop acceptance. [Gallery evidence](evidence/gallery-och41.md) retains commands,
checkpoint results and the boundary between local tests and release requirements.

The Controls gallery now composes a command button with live shortcut hints in a
native-managed tooltip. A mounted Here-context observer supplies the keycaps,
including shortcut replacement/removal and disabled state; the label platform is
independent of registration. Full OCaml tests/formatting and gallery build pass.
Its desktop walkthrough is authored and compiled but unrun. See the
[composition contract](design/command-binding-observations.md#command-button-tooltip-composition--2026-10-01).

The selection gallery now demonstrates independently loading and disabled children
inside connected or separated groups. Stable command owners render passive rich
labels; a loading Bold button remains focusable, while disabled Italic is
unavailable. Current-model reducer tests pass queued-request rejection and bulk
updates that preserve unavailable selections. Full OCaml tests/formatting and the
gallery build pass. The expanded desktop driver is compiled but unrun; native
AX/focus/geometry acceptance of this composition remains open.

Core/Bonsai now expose `View.with_hover` for Button, CommandButton and Link.
The independent observer preserves action/focus owners, uses generation-checked
callbacks and a bounded native event queue. A reproduced stationary-pointer
eviction bug is fixed by retaining observation state until accepted-node retirement.
The local checkpoint passes paired protocol fixtures, Core lifecycle tests, 498
native library tests (two existing skips), atomic admission, strict Clippy, full
OCaml tests/formatting and the gallery build. Seven focused native tests also pass, including command availability, Link
loading, ancestor gates, keyboard modality and modal trapping/recovery. The public appearance card displays hover; its desktop driver
is authored but unrun. A fresh staged installation also builds the independent
gallery against these public APIs and the rebuilt backend. This is TestPlatform
and build evidence, not macOS input or installed-consumer runtime acceptance. See the [hover contract](design/button-content.md#independent-hover-observations--unpublished-epoch-3).

The macOS adapter now projects busy state through `AXElementBusy` and queues its
change notification. A headless AppKit fixture passes for Button, Link and editable
text fields: Boolean type, enumeration, read-only busy state, retained identity,
ready/busy/disabled/recovery, existing properties and retired-node safety. It
creates no window or NSApplication. Strict Clippy, Rust formatting, full OCaml
tests/format and the rebuilt gallery pass. Eight scoped patches reconstruct exactly from
the pinned archive, with all 13 original source and two license hashes verified.
External AX notification delivery, public-gallery busy checks and VoiceOver remain
unverified; this is direct AppKit getter evidence. See the
[adapter record](../vendor/accesskit-macos/GPUIO.md).

Composed Links now expose focus-preserving loading in Core/Bonsai. The native
owner suppresses hover/pressed paint and activation while busy, retains Tab
policy, and rejects queued input even after loading ends. Disabled remains a
separate focus-removing policy. Paired Op60 adds one Boolean in unpublished
epoch3; both runtimes must be rebuilt together. The public appearance preview
has a loading toggle and an extended, unrun desktop walkthrough. Full protocol,
OCaml tests/format/gallery build and native checks pass locally (496 library
passes, two existing skips, five Link admission tests). These are local build
and TestPlatform results; updated Link physical/consumer runtime acceptance,
external macOS AXBusy validation and the black-window startup issue remain open.
See the [Link contract](design/composed-links.md#loading-policy--paired-unpublished-epoch-3).

The split control now hugs its content by default. A native regression reproduced
the prior stretched hover region highlighting both halves when the pointer was
in empty column space; the alignment repair and caller-override check pass.
Expanded TestPlatform coverage also passes tooltip-wrapped keyboard activation,
whole-pair disabled/inert/hidden closure, open-menu focus retention when removing
the primary, Escape and native owner cleanup. The library reports 495 passed/two
existing skips; full OCaml tests, formatting and gallery build pass. The preview
now has an explicit transparent resting background. A nonlaunch desktop preflight
still returns no CoreGraphics window list and false AX trust; no new OS/GPU/AX
or installed-consumer runtime acceptance is claimed.

The split-button [contract](design/menu-observation.md#chosen-split-integration)
now connects checked Core/Bonsai composition, paired Op70 under unpublished epoch3,
atomic native admission and native group hover/menu-held painting. Keyed part
slots preserve full caller keys and the surviving control across mode changes.
Native checks pass for two independent pairs, hover precedence, loading/disabled
primary actions, menu replacement, focus loss and reset: 494 library tests pass
with two existing skips, plus one admission test. Full OCaml/Rust workspace
checks, formatting, strict Clippy, the gallery and a fresh installed-gallery
consumer build pass locally. The public Controls preview has an authored but
unrun desktop walkthrough. Physical acceptance and the remaining lifecycle
matrix are still open. The private native group name is constant because GPUI
retains emptied group-name entries; its lexical stack isolates the pairs.

MenuButton now exposes optional native open-state observations and root popup
placement through Core/Bonsai. Protocol epoch3 pairs these changes across both
runtimes; unsupported older epochs are rejected. The Controls gallery demonstrates
subscription changes, disabled menus, submenu commands and placement. Native
lifecycle checks pass for snapshots, ordered transitions, focus loss, retained-node
culling and retirement. Expanded checks cover all placement sides/alignments,
three display scales, edge flipping/resize and render-time queue exhaustion;
493 library tests pass with two existing skips.
Full OCaml/Rust workspace checks, strict Clippy, formatting, the rebuilt gallery
and a fresh independently installed gallery build pass locally. The desktop
walkthrough remains unrun. See the [menu contract](design/menu-observation.md)
for pending root-scroll, shared-hover/split and physical acceptance work.

The Controls gallery now has eleven button appearance variants, semantic Link,
outline/compact/large/rounded controls, selected-versus-toggle separation and
plain/rich placed tooltips. The gallery builds. A production-View pointer/paint
regression verifies loading during a held press, focus retention, blocked release,
disabled precedence and restored activation; the native library passes 490 tests
with two existing skips. The new desktop driver is authored and unrun.
The [menu coordination contract](design/menu-observation.md) records the current
observation/placement implementation at that checkpoint. The later split and
hover checkpoints above supersede its missing shared-hover and callback status;
captured pointer gestures remain a distinct API.

The rich-label semantic wrapper now has an explicit stable identity, so GPUI can
emit its hidden decorative ancestor around type-erased content. Inert navigation
and carousel shields use the same identity rule; disabled tables use the disabled
prepaint scope. Direct wrapper and native layout/input/resource checks pass
(489 library tests, two existing skips). Physical AX validation remains open.
The selection gallery adds connected/separated groups, both orientations and
single-item geometry. Its desktop driver is extended but unrun; this is not
whole-family or milestone acceptance.

The [button content contract](design/button-content.md) now connects Core/Bonsai
constructors to native rich rendering, loading and focus policies. The reconciler
checks current per-button command state and fences queued loading/reference cycles
without disabling shared commands. This introduced epoch2 (now superseded by
epoch3 above); OCaml validates
the Welcome response before enabling submissions. The Controls gallery has a
shared-command rich/plain/busy/focus preview. Full OCaml/Rust tests, formatting, strict Clippy and a fresh independently installed
gallery build pass locally.
Physical macOS, installed-consumer runtime and full family acceptance remain open.
The follow-through adds AccessKit Busy metadata and a passing rich spinner/progress
lifecycle regression for busy motion, hidden/inert/transparent idle, reset/close and
retired SVG cleanup. The native suite passes 487 tests with two existing skips.
The public button driver is authored but unrun. The pinned macOS adapter's missing
`AXElementBusy` mapping remains an explicit release accessibility gap.

The [checkable navigation extension](design/checkable-navigation.md) connects
Core/Bonsai standalone Radio and optional checkable Tab order to paired protocol
operations and native rendering/focus. Checked activation retires the callback
without replacing the owner; explicit semantic groups support application-owned
selection. The public Controls gallery demonstrates rich/plain labels,
reverse/skip/reset order and disabled controls. The integration checkpoint passed
full OCaml/Rust workspace checks, formatting, Clippy and a fresh installed-gallery
build. Capability62 is paired across both runtimes; further mask growth requires
explicit protocol version design.

The subsequent native-only checkpoint passes 484 library tests (two existing
skips), adding modal trap/restoration, a 100,000-row logical list with bounded
described radios, eviction/remount fencing, session-close rejection and production
semantic-builder coverage. Final focused checks also verify button-state release
after GPUI processes destruction of the closed root. Real macOS keyboard/pointer/AX
and installed-consumer runtime acceptance remain open. This does not complete the
selection family or milestone.

The [button source review](catalog/button-review.md) records seven exact pinned
snapshots and distinguishes existing string/icon actions from remaining rich and
loading content, button focus policy, connected groups and split-menu coordination.
These gaps remain release work; the existing icon tests do not establish full
button-family parity.

[Rich checkable labels](design/control-labels.md) now connect checked Core/Bonsai
constructors, keyed radio overrides, paired capability61 and atomic native
admission/rendering. The public gallery toggles rich/plain labels while retaining
control identity. The production-View regression passes with the default test
stack for one-owner clicks, label ordering, disabled avatar fallback styling,
selection exclusion, spinner wakeups and cleanup. It also led to separating
nonrecursive presentation setup from tree traversal to reduce debug stack use.
Four native admission/fixture tests, the full Rust workspace and 477 native
library tests (two existing skips) pass; strict Clippy, the full OCaml
test/format/gallery build and a fresh installed-gallery consumer build also pass.
Radio reorder checks preserve label/disabled-state identity and the native focus
owner. Actual rich-label GPU/AX/public/installed-consumer runtime validation
remains open; the selection family and milestone are not
complete.

The managed radio group now exposes selected state and one-based position/set
size alongside its toggled value. Production semantic-builder tests include
disabled selected options; a TestPlatform key-dispatch regression preserves
rapid A→B→A requests before OCaml commits a value. The full native library passes
479 tests with two existing skips; strict Clippy, formatting and the gallery
build pass. This preserves the asynchronous request
contract rather than adopting the standalone primitive's checked-click no-op;
standalone radio composition and real macOS AX acceptance remain open. See the
[radio review](catalog/selection-review.md#radio-semantic-and-request-contract).

The [control appearance contract](design/control-appearance.md) now connects
Core/Bonsai View arguments, theme-resolved reconciliation, paired Op67/capability60,
atomic native admission and scalable painting. Native scene and transaction
checks pass for sizing, nested opacity, clipping, resets, rollback and ownership.
Production-View focus/lifetime checks, five Core expect tests and the full OCaml
test/format/gallery build pass. The public preview exercises custom/default
appearance, sizing, label order, mixed and disabled/inert states. Actual
GPU/AX/gallery/consumer acceptance remains open; this does not complete the
selection family. The full Rust workspace, 475 native library tests (two
existing ignores), strict Clippy and formatting also pass locally. A fresh
installed-gallery consumer builds. The new physical GPU/AX fixture links, and the
focused public driver has passing offline pixel-coordinate checks; both desktop
scenarios remain unrun.

The [selection review](catalog/selection-review.md) now maps nine pinned sources
and implements typed toolbar orientation plus command-based single/multiple
selection in the Controls gallery. Current-model reducer, paired codec and native
metadata/atomic cleanup checks pass, as do the full OCaml tests/formatting/gallery
build and 469 native library tests (two existing ignores). The focused desktop
driver is authored but unrun; rich-label/part-style and other selection-family
gaps remain explicit. This does not establish native visual or release acceptance.

Spinner and text-shimmer clocks now preserve visible layout time when a pending
frame callback arrives between preparation and paint. Both regressions fail
before and pass after the correction, including omitted-frame/reduced-motion
checks and Loop/Once shimmer. The full native library passes 468 tests with two
existing ignores. This is headless evidence; no new desktop or whole-application
performance acceptance is claimed. See the [spinner](design/custom-spinner.md)
and [text-shimmer](design/text-shimmer.md) contracts.

The [progress source review](catalog/progress-review.md) now has public
`View.progress_circle` with keyed center content and optional linear transitions,
paired Op66/capability59, retained native clocks, rounded/circular painting and a
Feedback gallery preview. The [contract](design/progress-presentation.md) records
semantic targets, interrupted motion, idle behavior, legacy reset and ownership.
The full native library suite passes 466 tests with two existing ignores, plus
atomic circle-child/reset admission. Production-View headless checks establish
center geometry/focus retention, target/display separation and close cleanup even
with a retained View. A two-window regression also repairs frame delivery
discarding an active transition interval between update preparation and paint.
A public progress gallery driver is authored, with offline ring-sampling checks;
its desktop assertions remain unrun. A physical progress fixture also links,
covering ring/rounded-fill pixels, opacity, clipping, native timing and macOS AX;
its runtime assertions are likewise unverified. Full Dune tests/formatting, the Rust workspace, strict native/protocol Clippy,
the gallery build and a fresh independently installed gallery consumer build pass.
Actual GPU/AX, public-gallery/installed-consumer runtime and whole-release
acceptance remain pending; these checks do not complete the catalog family.

The [loading source review](catalog/loading-review.md) documents spinner/skeleton
differences and required custom spinner icon/easing gaps. It also exposed and
repaired loading artwork disappearing inside inert subtrees. The production-View
regression fails before the fix and passes afterward; all 428 native library tests
pass with two existing ignores. Native GPU/AX acceptance of the repair remains open.
The [custom spinner contract](design/custom-spinner.md) now has public Core/Bonsai
constructors, Op65/capability58, atomic loading/icon ownership, separate mask
admission and a retained paint-driven native clock. The Presentation gallery
adds a scoped arrow icon, failed-icon fallback, easing and timing controls.
Core expect/protocol/native-admission tests and three production-View TestPlatform
checks pass, including queued image observations, legacy reset, inherited tint,
inert/hidden/transparent idle and window-close cleanup of never-painted leases.
The gallery builds and the full Dune tests/formatting checks pass. The full Rust
workspace passes; the expanded native suite now passes 449 tests (two existing
ignores), including queued-event retirement and independent shared-asset windows. Earlier mask,
clock and scene checks cover rotation reuse, quota boundaries and weak ownership.
A fresh installed-gallery consumer build also passes. A native spinner GPU fixture
is authored; its runtime assertions remain unverified. Real macOS GPU/AX,
gallery/installed-consumer runtime and whole-application resource
acceptance remain required; this is a local implementation, not release completion.

The unresolved [public canvas activation failure](evidence/canvas-activation-och17.md)
now has opt-in AX/queue/application traces and a passing TestPlatform regression
for scene replacement and callback retirement. This preserves the existing input
fences; it does not establish the cause or resolution of the hosted macOS failure.

The unresolved [initial black window](evidence/window-startup-och17.md) now has
a bounded startup-capture tool that observes only the launched app's windows
without AX/input/activation. Offline historical-frame analysis and process cleanup
checks pass. This session cannot enumerate the macOS desktop; the tool's preflight
stops before app launch. Fresh startup capture and a production fix remain pending.

The [rich avatar fallback](design/avatar-fallback.md) now has checked Core/Bonsai
and group-item constructors, native selection and independently owned visibility.
A TestPlatform lifecycle check passes for raster/SVG selection, retained fallback
assets, hidden nested tween/program deadlines and first-frame SVG size failure/recovery.
An expanded native image fixture is authored for GPU clipping, GIF playback,
AX ownership and disposal; its runtime assertions remain unverified. Real
GPU/AX/gallery acceptance and current-head CI remain pending; the family is not
complete.

The [Rating source review](catalog/rating-review.md) records click/hover/size
differences. Independent active/outline colors now have a typed public
[appearance API](design/rating-appearance.md), theme resolution and paired native
updates. The gallery exposes colors, size, maximum and request-policy controls.
Core/bridge checks pass locally; new native GPU and public-gallery assertions
are authored but unrun. Current consumer/hosted/native acceptance remains open.

[Hosted run 36791905054](evidence/milestone-07-ci.md) has finished: Linux passed;
macOS passed full table history but failed public canvas/date-picker checks and
was later cancelled. A local picker-test readiness correction and larger macOS
job budget await runtime/hosted validation; this is not release acceptance.

The [avatar source review](catalog/avatar-review.md) now includes an implemented
`Avatar_group` API and team gallery preview. Four Core tests, the full local
OCaml build/test/format check and a fresh installed-consumer build pass. Native
group geometry, paint, keyboard/AX and resource acceptance are pending desktop
access. The fixed identity palette now has a passing 24-pair numerical contrast
audit; rich fallback native acceptance remains pending; the avatar family is not yet fully accepted.

The [rich Form collection](design/form-composition.md) now has typed items,
validated column placement, rich slots, shared/per-item label policy and a trailing
footer, with a public Text editing gallery example. Core validation/reconciliation
and full local OCaml build/test/format checks pass. The native gallery driver
stopped at an unavailable macOS Accessibility preflight; **Form geometry, native
keyboard behavior and fresh installed-consumer runtime acceptance remain pending**.


Current checkpoint: milestones 1–6 are merged. Milestone 5's
[PR #13](https://github.com/dakotamurphyucf/gpuio/pull/13) merged at `936fb7d` after
[required macOS/Linux CI](https://github.com/dakotamurphyucf/gpuio/actions/runs/36312697654)
passed on `473407c`; all twelve tickets are Done.

[Typed native grid placement](design/native-grid-location.md) now exposes
atomic row/column locations with validated signed lines, spans and Auto edges.
Independent codec/admission checks, full Dune/Rust suites and strict Clippy pass
locally. A background macOS fixture passes 20 geometry/GPU cases plus native
hover replacement/restoration, absolute placement, reset and cleanup. This is the
layout prerequisite for rich Forms; the collection API, public gallery/consumer
integration and hosted validation remain open.

The [Settings composition](design/settings-composition.md) now supplies bounded
Core metadata and a controlled Bonsai sidebar, native split, managed groups,
responsive field layout, rich slots and explicit reset requests. Deterministic
tests cover search/navigation, current-state resets, full-length identities,
responsive control retention and page/row lifetimes. The page-visit test exposed
and helped repair captured virtual-list effects surviving a generation revisit.
Typed Boolean/choice/native-field helpers now pass identity/dispatch/semantic
checks. Public text/numeric examples pass native mount-seed behavior and guarded
reset commands. Atomic numeric mount seeds now recover unfinished drafts
independently of committed values, with codec/admission/native command evidence.
A dedicated Settings gallery now passes scoped keyboard editing, responsive
identity, search/page draft recovery and export failure/cancellation checks.
A fresh installed consumer passes those checks plus long-choice keyboard selection
and Unicode paste. Placement-scoped observations now make unmounted editor resets
safe; group reset controls, export validation and real native Save/Eio readback
also pass scoped repository and fresh installed-consumer checks. Expanded repository and consumer checks
pass pointer/keyboard resizing, custom-policy resets, focused-row retention and
eviction across 48 groups, and independent two-window keyboard edits. A native
warm-row overlap fix repairs navigation after focusing a tall row. The
[initial second-window AX focus mismatch](evidence/window-accessibility-och17.md)
is repaired with a scoped adapter initialization patch and passing two-window
OS input/focus checks. Native [disabled subtrees](design/disabled-subtrees.md)
now preserve discoverable controls while blocking input, stale AX/menu actions
and active gestures; the public Settings gallery and a fresh installed consumer
pass disabled-state and retained-identity checks. Actual US dead-key checks now
pass composition retention across layout/theme/size changes, guarded and partial
resets, commit/undo/redo and text/numeric owner retirement. They exposed and
helped repair a window-level macOS input-context leak into the next editor.
Rapid page/group navigation also exposed a reveal against estimated list heights;
the destination now remains requested until its actual row is painted, with
supersession, user-input cancellation and a scoped native sparse-row regression.
The installed-consumer field matrix additionally passes all six standard control
kinds across 24 variant/size/layout/theme cases, numeric bounds and saved-value
remount/reset. Together with deterministic/native reset guards, the
[acceptance map](design/settings-composition.md#local-source-row-acceptance-map--2026-09-30)
now establishes local Settings source-row equivalence. Broader IME, other catalog
reviews and release acceptance remain open.

The [mounted native binding observer](design/command-binding-observations.md)
now connects paired transport, Core View callbacks, Eio delivery and a Bonsai
pending/latest-value adapter. A focused local macOS test passes actual widget
bindings, child-driven composition changes, context/config/epoch replacement,
coalescing, visibility recovery and cleanup. Core and native admission/lifecycle
checks pass, and Bonsai tests preserve child state while fencing stale observation
effects. The public gallery and a fresh installed-library consumer now pass
20 live binding/name/identity cases, actual OS Copy and command invocation,
query/config/page retirement and unchanged-epoch silence. Expanded native matrices
pass nested shadowing/phase precedence, modal focus/restoration, sparse retained-row
suspension/eviction, bounded-work recovery and independent-window close/reuse.
The Kbd source row is locally a functional equivalent; Settings, the remaining
catalog reviews and release gates are still open.

[Typed keyboard labels](design/keyboard-labels.md) now format validated shortcuts
for macOS/Linux, supply spoken names and filled/outline/plain keycaps, and expose
ordered command declarations. Core/full Dune and a fresh installed consumer pass
native geometry, GPU appearance, identity and explicit registration/disabled/routing
checks. Effective native focus/context lookup is now implemented with scoped native
and public gallery/consumer evidence above; whole-release acceptance work remains.

The [rich Description list](design/description-lists.md) now provides validated
column/span packing, rich term/value slots, both axes, label widths, sizes, borders
and separators while preserving keyed native children. Four Core tests and full
Dune pass; repository and fresh installed-consumer checks pass 75 layout/semantic
order cases, 34 GPU cases, OS actions, draft/control retention and slot/page
retirement. The source row is locally a functional equivalent. Settings,
other catalog reviews and OCH-17 release gates remain open.

The [rich Bubble/Message adapters](design/chat-composition.md) now provide typed
surfaces/reactions, optional message slots, independent alignment and Ghost inset
metadata. Core/full Dune checks and a fresh installed consumer pass scoped native
geometry, GPU paint, keyboard/pointer actions, editor retention, streaming and
cleanup. The managed transcript exposed a list/ancestor double-scroll defect;
its [repair](evidence/scrolling-och11.md#milestone-07-managed-list-inside-an-ordinary-scroller)
passes failing-before/native and actual desktop regressions. Native list history,
selection/editor lifetimes, strict Clippy and 413 unit tests pass. The two source
rows are locally functional equivalents; Settings and the wider
catalog/release gates remain open.

The [rich Tag adapter](design/presentation-tags.md) now supplies direct rich
children, semantic/custom palettes, outline, size groups and native hover styles.
Core checks pass; a fresh installed consumer passes 28 palette/outline GPU cases,
native hover override/unset, size and reorder geometry, rich-only/empty content,
30 OS actions and slot/page retirement with preserved caller state. The source
row is locally a functional equivalent; wider catalog and release gates remain open.

The [rich Alert adapter](design/presentation-alerts.md) now supplies typed variants,
sizes, Card/Banner layouts, optional rich slots and a localized native close button.
Core/full Dune checks pass. The repository gallery and fresh installed consumer
pass 20 theme/variant/banner cases, eight size layouts, 21 OS body actions,
disabled close, keyboard/pointer dismissal and control retirement. The consumer
also passes GPU tint/border checks. The source row is locally a functional
equivalent; the presentation family and wider release gates remain open.

The [rich Marker adapter](design/presentation-markers.md) now provides typed
Plain/Separator/Border composition, icon/content slots and native Spinner/Shimmer
loading. Core validation/style/identity checks and the focused macOS gallery pass.
A fresh installed consumer passes 18 theme/variant/icon cases and 21 OS keyboard
actions, GPU text/rich/static paint, opacity refinement, reduced-motion recovery
and page/slot retirement. The source row is locally a functional equivalent;
Settings remains in the presentation review, alongside other
catalog and OCH-17 release gates; Bubble/Message evidence is recorded above.

Native [animation opacity factors](design/animation-opacity-factor.md) now supply
the styling primitive for rich Marker loading. The bounded factor multiplies
base/interaction opacity without adding a layout wrapper; absolute opacity remains
unchanged. Independent bytes, validation and atomic rejection checks pass locally.
A background macOS GPU fixture passes base/state/ancestor/descendant composition,
layout, native pulse timing, reduced-motion idle and owner teardown. Rich Marker
composition now has public gallery/consumer evidence above; wider release
acceptance remains open.

The [rich Attachment adapter](design/presentation-attachments.md) now composes
typed status, size, axis, media/content/action slots and whole-card activation.
Core checks cover retained identities/current callbacks, slot retirement, shimmer
inheritance, image-only opacity and theme-relative alpha. The focused public macOS
gallery passes 20 theme/layout/status cases, real keyboard and pointer actions,
shielded action gaps/disabled buttons, decoded-image sizing and page teardown.
A fresh installed consumer also passes decode-failure recovery, direct card
pointer/accessibility activation and zero image/source counts after departure.
Installed-consumer GPU checks additionally pass title motion/static restoration,
image/overlay/description tinting, pending dashed paint, reduced-motion recovery,
horizontal scroll and vertical parent routing. They exposed an invisible default
dark highlight, repaired with an explicit white highlight. The Attachment source
row is now locally validated as a functional equivalent; other catalog and release
gates remain open. This does not establish Linux GUI, screen-reader or application
performance acceptance.

Native [aspect ratio](design/native-aspect-ratio.md) now exposes preferred
proportional layout through validated styles, paired transport and capability bit
50. Background GPU checks cover resizing, padded percentage widths, explicit
dimensions, transferred min/max constraints and state/unset behavior. The public
gallery and a fresh installed-library consumer pass twelve theme/ratio/width
cases, thirteen keyboard actions, native identity/focus retention and page
remount. This supplies the square-media primitive used by richer attachments;
the broader release gates remain open.

The [text-shimmer adapter](design/text-shimmer.md) now connects Core/Bonsai,
validated live transport and native retained rendering. The mounted background
fixture passes actual glyph paint, wrapped selection/copy, foreground spans,
search underlays, native source accessibility labels, application-theme overrides,
visibility/opacity/clipping, independent windows and owner disposal before app
shutdown. An equal-text update regression also verifies that owners share the
current Tree source allocation without restarting the effect. These complement the independent
painter/clock matrices; they do not establish foreground keyboard/IME, screen-reader
or whole-application idle/performance acceptance. A public gallery preview is
implemented. Native retained-tab, responsive-branch and managed-row
pause/eviction/remount checks now pass. They exposed and helped repair a hidden
search-scope redraw loop and last-scope native-visibility recovery; the full
mounted highlighting regression also passes. A shared per-window overlay budget
now has native boundary, static-fallback, pause/recovery and independent-window
evidence. Public normal-launch and independently installed consumer checks now
pass eight theme/width/direction combinations, real keyboard Unicode Copy and
effect toggling, identity, one-shot playback, reduced motion and page remount.
They exposed a native clock defect that excluded visible layout time; a
failing-before deterministic regression and passing public checks cover the
repair. Marker integration now has scoped evidence above; measured application
performance remains open. No shimmer capability is advertised yet.

The [finite style-value audit](evidence/style-finite-values-och41.md) now covers
seven keyword sets with pinned native GPUIX sources. Grid count/minimum and
text-decoration replacement differences are explicit; paired OCaml/Rust bytes,
atomic validation and native refinement checks pass locally without GUI windows.
Fourteen further [native keyword sets](evidence/style-native-aliases-och41.md) now
record aliases and state-local unset semantics against GPUIX's exact GPUI
submodule. Alignment helper/refinement and layer-merge tests pass. The [numeric/shorthand audit](evidence/style-numeric-policies-och41.md) records
limits, percentage and Auto/sign semantics, byte/count limits and ordered
composition for 47 more field rows. Independent OCaml/native validation checks
and atomic rejection pass; specialized-root behavior and release gates remain
open.

Solid and dashed border patterns now have a typed `Style.Border_style` API and
negotiated field 67/bit 49. The [border contract](design/native-border-styles.md)
records paired bytes, atomic invalid-value rejection, state-local resets and
72 background GPU cases covering widths, radii, resizing and individual edges.
Native hover/press transitions and idle teardown pass. The public style-gallery
card also passes 16 normal-launch AX geometry/identity/state cases in both the
repository app and a fresh installed-library consumer. Empty integration now also
passes its scoped native consumer checks, as does pending-border paint in the
Attachment adapter. Whole-release acceptance remains separate work.

The richer [separator composition](design/presentation-separators.md) now exposes
both axes, labels, solid/dashed patterns and independent slot styling without
changing the original helper. Core identity/refinement checks and a fresh installed
consumer pass 32 native geometry/state cases plus twelve long-label clipping/reset
cases. Captured pixels verify the clip in both themes. The Separator source row
is locally validated as a functional equivalent; wider release gates remain open.

Ordinary two-axis containers now preserve diagonal scrolling. The new native
[scroll regression](evidence/scrolling-och11.md#milestone-07-parity-two-axis-containers)
reproduced a dropped Y component and now passes precise/discrete diagonals,
boundary propagation, hovered-axis changes and retained-owner teardown alongside
the existing nested transcript/composer/popup/modal suite. Physical trackpad,
public-gallery and Linux desktop acceptance remain separate.

Milestone 07 is in progress. OCH-41's public
catalog now has an initial [presentation behavior review](catalog/presentation-review.md).
The missing status-bar center region is implemented and passes 48 actual macOS
gallery theme/size/slot combinations with geometry, keyboard and identity checks.
Overlay badges now provide capped counts, zero hiding, dots and SVG icons while
preserving the existing text-chip helper. Local native checks pass 36 kind/size/
theme cases and 72 pointer/Return activations, uncapped AX labels, underlying
control identity and scoped asset cleanup. Enhanced labels now supply inline
secondary text, Unicode-aware prefix/all-match coloring and display masking.
Local gallery checks pass 48 theme/width/configuration cases with real keyboard
copy, native identity, masked AX source and cleanup. Group boxes now expose plain,
filled and outline body panels plus independent header/body/footer styles, retaining
the original card default. Local native checks pass 64 layout/theme/style/slot
cases with checked-state/identity retention, 128 pointer/Return actions and page
teardown/remount; the combined core gallery and full Dune checks pass. The
presentation family remains only partially reviewed.
Rich empty-state slots now add independently styled media/title/description,
content and extras while retaining the string helper. Core tests and a native
16-case gallery matrix pass layout, wrapping, action/focus identity and teardown;
a scoped decoded-image case also passes. The [Empty review](catalog/presentation-review.md#empty-state-rich-slots)
records the precise coverage and a functional-equivalent source mapping. A fresh
installed-library consumer now passes 16 layout, ten border and twelve proportional
typography cases with real keyboard actions, retained state/focus and scoped cleanup.
The [composed-link review](design/composed-links.md) now records validated rich
content, retained ownership and signed Tab policy through OCaml/Rust codecs,
native admission and View/Bonsai rendering. Native and public-gallery checks pass
image/avatar/loading/animation content, inherited styles, outer highlight scopes,
one action/focus owner and scoped cleanup. A loading-child focus-entry defect and
a nested SDK Tab-boundary defect have failing-before regressions and passing fixes.
The gallery covers eight base cases plus rich previews with 42 native actions.
Measured scroll reveal, fixed clipping, range thumbs, modal restoration and nearest
extension focus ownership also pass locally on unlocked macOS. Earlier locked-
desktop waits are excluded from acceptance.
Composed links now negotiate bit 48 (`CAP_LINKS`); highlighting remains separately
unadvertised. Paired Hello fixtures and session rejection tests cover the new
required mask. A fresh installed-library consumer now passes all 42 Link native
activations, focus/Tab/reveal, disabled recovery and scoped cleanup checks. Both
Link source rows are locally validated functional equivalents; whole-release
gates and the other presentation modules remain open.
The remaining presentation modules have a
[pinned behavior/gap review](catalog/presentation-gaps.md), including nested settings
fields. These implementation plans and source snapshots do not mark the families
accepted; settings composition
remain, alongside the other catalog reviews.
The [ordinary text-span API](design/text-content.md) now supplies bounded
foreground runs through atomic bridge updates and View/Bonsai reconciliation.
Local checks cover native GPU paint, wrapping, selection/copy and source AX labels.
The public label helper and gallery build on this same text primitive.
OCH-41's public
[Component Studio](../examples/gallery/README.md) now has twenty-four preview sections, including Settings,
including canvas, images/icons, charts, native motion, responsive layouts, native extensions, input/transfers, input observations, desktop services and styling details. The [gallery evidence](evidence/gallery-och41.md) records
local macOS interaction, geometry, gallery expect tests, formatting and structural
catalog checks, separately from earlier native document/Clippy regression checks.
The current catalog maps every required v1 family to a gallery page. Detailed
behavioral parity and release gates are still pending. The earlier 23-section
walkthrough passes on both the repository application and a fresh independent
consumer of installed public libraries. The consumer backend builds with its
independent lockfile; this does not establish clean-machine distribution. The
independent [Signal Studio consumer](evidence/signal-studio-och29.md) also passes
its self-test and full AppKit input/layout/lifetime walkthrough locally. The
[event audit](catalog/gpuix-events.json) records validated input contracts and remaining subtree
highlighting and diff-control gaps. The [highlighting foundation](evidence/subtree-highlighting-och41.md)
now has paired validated configuration, bounded text/range projections, an
owned background-work pool, retained scope declarations, validated observation
routing, bounded retained-tree collection, a GPUI executor service and an
independently verified shaped-text highlight painter. Mounted ordinary/selectable
text now passes GPU painting, queued observations, cosmetic reuse, source and
visibility updates, and unmount cleanup. Installed code/diff/source-mode pages
now pass rounded GPU highlights, selection precedence, native page/collapse changes,
streaming revision replacement and owner disposal. Prepared Markdown headings,
formatted/inline-code text, fences, tables and wrapped paragraphs now also pass
focused native GPU/selection/streaming/collapse/cleanup checks. Declared custom
text, literal HTML and image placeholders now share that painter; decoded images
remove placeholder matches without changing document revisions. Focused native
checks now also pass for 100k logical-list row reuse and independent windows;
scroll clipping, retained tabs/disclosures and responsive branch changes now also
have focused native evidence. Animated navigation now verifies selected-route
counts and pixels, interrupted slides, reduced motion and transition disposal.
Ordinary styled roots now pass native hover/pressed/focus visibility, hidden-base
overrides, display-none and restyle checks. A pinned GPUI core patch now restores
pressed-hide elements on release, cancels stale activation and passes controls
regressions. Remaining style parity and application performance/resource
acceptance are still open. sRGB/Oklab gradient interpolation now has an explicit public API, paired bytes,
native validation, GPU midpoint checks and a public gallery toggle; see the
[gradient evidence](evidence/gradient-color-spaces-och41.md). The public Find &
highlight gallery now demonstrates
live queries, ranges, nested exclusions, selected matches and growing/collapsed
Markdown, with focused local native interaction evidence. Targeted Hebrew/Arabic
and mixed-direction GPU checks now pass after fixing reordered-glyph range
geometry; full typography/input acceptance
remains broader than these cases. The gallery page count does
not imply parity. Document selection colors now inherit and restore defaults with
source/Markdown GPU evidence; the [selection audit](design/selection-style-audit.md)
now records inherited document selection-disable behavior, restored native Markdown
drag/copy through the window selection layer, retained focus-trap Copy isolation,
and passing document/UI/editor/control regressions. Ordinary text now participates
in window selection with scoped Copy, source retirement, whitespace preservation
and mapped truncation. The new native fixture covers cross-node drag, keyboard
Shift-click, reorder and hidden endpoint retirement. A mixed ordinary/Markdown
fixture also checks focus-independent Copy, local Select All, source replacement
and endpoint removal. Ordinary selection also passes independent-window Copy,
close/reopen release and managed endpoint eviction/rematerialization checks.
Guarded interior-row eviction now also verifies surviving endpoints, released
payloads, current-generation rejoining and selected-source retirement.
Ordinary pointer projection now preserves extended graphemes and uses shaped
visual cells, with focused left/center/right, Hebrew/Arabic, soft-wrap and LF/CRLF
Copy evidence. Native word/caret hits and keyboard-to-Shift-click anchors now use
that geometry too, with focused accented/Hebrew/Arabic and whole-grapheme evidence.
Two independent Markdown documents now pass shared Copy, local Select All, reorder,
source replacement, unmount and node-generation reuse checks. Measured macOS
pointer Copy source/code/table controls also pass with selection disabled, and
Markdown single/double/triple-click drags respect inherited disable. Broader
typography/input, mixed document modes, virtualization and the complete accessibility
matrix remain open. General input regions now have mounted Core/Bonsai/native
integration, native edge-case and public gallery evidence; remaining consumer/
release acceptance stays open in the [input ledger](evidence/input-observations-och41.md). Start ellipsis and the complete cursor vocabulary now have
paired codec, native validation and focused gallery evidence. OCH-17 is also in progress: the
[native document accessibility repair](evidence/document-accessibility-och17.md)
now exposes body text, read-only source/code, keyboard/AX inline-link activation
and distant-link reveal. Parsed heading levels now reach macOS AXValue; Markdown
tables expose row/cell structure with distinct identities and indices. The focused
gallery passes level 1, wrapping-table counts/Unicode reading order and repeated
collapse/remount. Table header queries now reuse painted cell/container identities;
Markdown and 100k logical-row managed-table regressions pass, including far-row
navigation and hidden-header retirement. Rich text/code and safe image-placeholder
links now have one accessible name/action, ordered text, keyboard focus and queued navigation in the public
fixture. Direct macOS AX focus now selects and reveals rich and ordinary links
without activation, using the document's native focus owner and guarded current
presentation. The full document walkthrough, focused retained-collapse check,
native ownership/reset regressions and 225 Base text tests pass locally.
Decoded image alternatives now have named Image nodes, single linked targets and
silent decorative semantics in a public gallery preview. The explicit-empty-text
parser defect is fixed; exact native reading order, focus/activation, collapse and
remount pass with 226 Base text tests and seven native parser tests.
The dark Markdown table-body contrast finding is also repaired: the native
adapter supplies the document surface independently of Base's global theme.
Actual GPU checks cover both table render paths, repeated appearance switches,
painted colors and contrast; full document/selection/highlighting and public
gallery regressions pass, with light/dark screenshots inspected. Custom-control
accessibility, selection/ranges, complete
table and screen-reader behavior remain open. The Base initial-render failure
reproduced on unchanged `dc25013` is now fixed by applying the initial selection
setting at keyed-state construction. Its unchanged threshold and all 224 text/
selection tests pass; application idle/performance budgets remain separate. The post-reset collapse race now has a deterministic
native regression and a generation-aware interaction repair. The full document
walkthrough passes again; earlier fence-reveal timeouts remain recorded without
an independently established cause. No full document
screen-reader or Linux desktop acceptance is claimed.

Deferred overlay, tooltip/hover-card, toast/stack and context-menu highlighting
now passes native count/GPU/lifetime checks. Floating-panel visibility preserves
its anchor; role swaps and child replacement reject obsolete samples. Toast stack
state styles now apply consistently with individual notifications. The full
native controls regression passes. See the [deferred visibility evidence](evidence/subtree-highlighting-och41.md#deferred-surface-visibility);
document wrapper hover/press/focus visibility also passes source/Markdown GPU and
retention checks, with an active-only hitbox correction in the pinned GPUI patch.
Native diff gutter folding also passes source-byte geometry, hidden-row painting,
selection precedence and result-reuse checks; folded/scrolled-out byte-range lookup
now rejects positions that have no laid-out text. Remaining style/diff API parity,
catalog and release gates are still open.

Diff preparation now records bounded per-file metadata with shared path labels,
paired old/new line coordinates and exact payload byte ranges. File boundaries
close the preceding hunk; a native two-file test verifies that folding preserves
the next header and its highlights. This is a foundation for the pending per-file
collapse, line-limit/show-more and richer callback APIs, not completion of them.
`Document.Diff` now has validated configuration/event domain values, paired
standalone OCaml/Rust codecs and tested native managed/controlled state. Additive
live transport, Core/Bonsai callbacks, configuration epochs and source-revision
checks now have integration tests. Controlled collapse and preview settings now
drive a mounted native editor with mapped selection, navigation, bounded pages,
canonical search/raw return and additional shared-pool memory admission. Native
GPU checks verify projected highlights, selection, hunk folding and owner disposal.
Show more now passes native pointer/keyboard/macOS accessibility activation,
managed/controlled observations, focus repair and retired-action rejection.
Rich line observations now pass native pointer/Enter/toolbar-AX checks for exact
Unicode/CRLF payloads, old/new coordinates, drag/gutter exclusion and installed
revision provenance. Per-file gutter controls and metadata now pass native
pointer, Tab/Space/Enter, macOS AX, managed/controlled state, callback replacement,
scrolling alignment and focus cleanup checks. Source text remains selectable in
the same editor. Filename-based syntax now uses separate old/new language contexts,
bounded background work and complete diff-color fallback. Word emphasis pairs
equal-length replacement groups and preserves syntax styling when disabled. The
public gallery and an independently installed consumer now pass focused native
diff controls, line events, streaming and theme/size/reset checks. Wide headers
now pass actual horizontal-scroll geometry/clipping checks, and a native selection
regression fixes file-button presses clearing selected source text. Streamed
selection/copy and bounded-page focus retirement also pass. Combined
gallery/consumer, performance and release gates remain; see the
[diff controls evidence](evidence/diff-controls-och41.md).

The macOS-first/Linux-deferral policy [PR #15](https://github.com/dakotamurphyucf/gpuio/pull/15)
merged at `af7f6f0c0f9db1c64a8591d9e9a078aa73eacdec` after both required jobs in
[CI run 36453260976](https://github.com/dakotamurphyucf/gpuio/actions/runs/36453260976)
passed. These are the policy PR's checks, not hosted acceptance of the new gallery.

Milestone 6's [PR #14](https://github.com/dakotamurphyucf/gpuio/pull/14) merged at
`bdbae672c97b046fca5d7e0a0f5bb779e24cfd01` after
[required macOS/Linux CI](https://github.com/dakotamurphyucf/gpuio/actions/runs/36432631460)
passed on `a7a514aa6a8162142be5e6e384797d8c9bd58657`. Source, CI merge-ref and
merged main have the same tree. All four M6 tickets are Done.
The [Linear project](https://linear.app/ochat/project/gpuio-8bd4e30f319d) records
ticket completion against those gates. Its four deliverables are:

- OCH-27: typed desktop identity/packaging, readiness-aware link routing,
  native document metadata, OS file open/reveal and Linux private-bus arbitration.
- OCH-28: application-scoped OS notifications, explicit permission/capability
  queries, owned receipts, replacement/dismissal and stale-safe actions.
- OCH-40: seven native chart families and mixed layers, revisioned bounded
  datasets/preparation, explicit sampling, native interaction and an accessible
  original-data table. Native windows/list retention and 10k/100k streaming are
  measured separately from the combined application.
- OCH-29: Signal Studio combines a public OCaml canvas, chart and independently
  packaged native component with documents, links and notifications. Responsive
  input, Full/Reduce motion, installed-library consumer builds and repeated
  command/resource/window lifetimes pass locally.

Consolidated local format, Dune/Rust tests (719 Rust tests), strict lint, all 18 M6
native/build/private-bus/OS stages, and a fresh public consumer's self-test and
complete workload pass on macOS 14.5 arm64. Required hosted macOS/Linux build,
unit tests and lint passed in the first
[consolidated run](https://github.com/dakotamurphyucf/gpuio/actions/runs/36377185296).
That run found test setup/sampling failures; the evidence ledgers record their
locally validated fixes. Use PR #14 for final hosted gate results, not that initial
run or local passes alone. The [M6 delivery matrix](milestone-6.md) links
current contracts, examples and evidence. Linux GUI/compositor validation is not
implied by compilation or private-bus fixtures.

Owner decision, 2026-09-28: milestone 07 is now a macOS-first v1 release. OCH-41
retains the full component gallery/catalog; OCH-17 retains macOS validation,
performance/resource budgets, documentation and clean-machine distribution.
Linux builds/unit/private-bus/consumer checks remain required and graphical
smoke remains informational. Full Linux desktop qualification has moved to
[OCH-47](https://linear.app/ochat/issue/OCH-47/qualify-linux-x11wayland-desktop-behavior-and-distribution-after-macos)
in deferred milestone 07b; it does not block M7, M8 or ongoing feature work.
Read the [platform release policy](platform-release-policy.md). No remote Linux
machine or local VM is required now. Earlier references below assigning full
Linux release acceptance to OCH-17 are superseded by this decision.

The entries below preserve earlier implementation checkpoints. Their pending-work
statements are historical; use the delivery record above for current status.

Updated 2026-09-26. Milestones 01 and 02 are merged, including native text editing,
controls/interactions and declarative animations. [PR #10](https://github.com/dakotamurphyucf/gpuio/pull/10)
merged at `17e863279bff25253edf47f449c04cd9aee5e867` after the required macOS and
Linux checks passed. Milestone 03 / OCH-13 implements keyed collections, paging and managed virtual
lists in [PR #11](https://github.com/dakotamurphyucf/gpuio/pull/11). See the [managed-list design](design/managed-lists.md)
and [local acceptance evidence](evidence/managed-lists-och13.md). The managed
component, paging, native interactions and full-history retention tests pass
locally and in hosted validation at `2c2063b`. See PR #11 for the final checked
head and merge. CI run 36056171245 also passes X11 list checks; Wayland stops
at the existing combobox clipboard failure before reaching them. Final review
adds a second complete OCaml 100,000-row traversal, also passing locally.

Milestone04 is implemented in [PR #12](https://github.com/dakotamurphyucf/gpuio/pull/12): revisioned
streaming documents and native Markdown/code/diff; independent windows, retained
tabs and split panes; and a polished agent-chat reference application. The app's
public integration and external macOS AX/keyboard/picker scenarios pass locally.
See the [M4 evidence ledger](evidence/agent-workspace-m4.md),
[ownership design](design/agent-workspace.md), and [runnable demo](../examples/agent_chat/README.md).
The full consolidated local build, suites, native regressions, Clippy and format
checks pass. The evidence ledger records hosted results; PR #12 records the final
checked head and merge status. Full Linux GUI acceptance remains OCH-17.

OCH-46's combined macOS workload now passes with 100k source nodes, 100k table
rows, canvas and native extension mounted in four windows during streaming.
Native keyboard input, bounded accessible rows/cells, painted animation without
additional OCaml transactions, and repeated window/canvas cleanup are measured.
Read-only runtime diagnostics distinguish serialized traffic and owned resources
from clock polling and total/native memory. Full/Reduce responsive checks also
cover settings-sheet resize, saved values and nested Escape/focus restoration.
See the [combined evidence](evidence/agent-chat-m5.md#combined-streaming-large-artifacts-and-cleanup).
Consolidated local Dune/Rust suites, native checks, all 15 chat walkthroughs and
a fresh staged extension consumer now pass. [PR #13](https://github.com/dakotamurphyucf/gpuio/pull/13)
records required hosted macOS/Linux results, the checked head and merge state.
The [milestone handoff](milestone-5.md) maps all delivered families and ownership
contracts to current source and evidence.

OCH-37 now has compiled and locally tested Core models for bounded navigation
history, single/multiple disclosure and pagination. Tests cover route replacement,
back/forward/branching, disabled and stale requests, collection/page-count shrink,
128-entry navigation, 4,096-item disclosure and 10,430 bounded pagination partitions.
Core/Bonsai panel/disclosure/accordion bindings and initial native macOS checks now
pass keyboard/expanded accessibility state, nested focus restoration, retained and
unmounted editors, marked-text isolation and hidden focus-scope cleanup. A small
vendored patch to the unchanged accesskit_macos 0.26.3 exposes expanded state;
both native backend build paths use it. Public breadcrumb/pagination compositions
now pass bounded-model/reconciliation tests and native AppKit current descriptions,
keyboard/AX actions and focus retention. The Navigation Lab also verifies retained
Unicode drafts, independent lazy Bonsai lifecycle and Eio data-scope cleanup.
The initial sidebar adds grouped/nested destinations, independent expansion,
icon/offcanvas modes, scoped icons, context commands and current-link semantics.
Local public macOS AX checks and screenshots cover its collapse modes; native
regressions also cover custom disclosure headers and retained hidden popup scopes.
Native sidebar width transitions now pass public macOS geometry, interruption and
reduced-motion checks. Retained offcanvas content now slides out on either side
while inert: native input/AX access stops immediately, editors survive, and nested
animations/popup scopes suspend. GPU and public screenshot evidence verifies paint
continues during exit. Navigation now has tested bounded native transition state
and a mounted Core/Bonsai presenter, including reversal from painted positions,
retained native controls, destination focus, outgoing GPU paint and immediate
removal. Native nested/modal, IME, pointer/keyboard exit gating and a full 128-page
workload with resize also pass, alongside the public example. Four retained editors
fit within unchanged editor quotas; history bounds do not exempt native resources.
Four-edge sheets and alert-dialog adapters now share the existing modal focus and
asynchronous dismissal infrastructure. Native macOS checks cover edge geometry,
clamping/resize, late hover styles, editor identity, focus restoration and nested
alert backdrop blocking. The public Navigation Lab includes drawer/confirmation
flows; its latest validation is recorded in the evidence ledger.
Interactive hover cards now expose a separate nonmodal Dialog role while reusing
native tooltip timing, retained content and placement. Local native tests cover
Tab/pointer/IME/Escape, accepted controlled close, anchor restoration and timer
cancellation; the Navigation Lab includes a contributor preview. Existing tooltips
retain their separate help semantics and grace clock.
Carousel now has a tested Core selection model, paired envelopes, Core/Bonsai
constructors, bounded default pagination and native admission/request dispatch.
Mounted horizontal/vertical presentation reuses retained pages; local macOS checks
cover GPU transition geometry, retained editors and focus preservation/handoff.
Native keyboard and auto-advance scheduling now pass local tests for child-editor
key isolation, pause/resume, clipping/window activation, one pending proposal,
no idle frames and teardown. Native wheel bursts now pass axis/cancellation,
momentum fencing after accepted selection, missing-end fallback, nested scrolling,
reduced-motion input and disposal checks. Native pointer/GPU checks now cover
axis locking, two-page preview, capture/rebinding, snap/accepted retargeting,
in-flight grabs, child-control priority and lifecycle/foreign-capture cancellation.
The public Navigation Lab now verifies native requests through Eio/Bonsai, AX/current
metadata, native auto-advance and explicit unmount leases while Bonsai/data remain
alive. Native marked-text and nested-popup checks now pass focus handoff, hidden
input rejection, IME-first Escape, editor key isolation, popup focus/hover pause
outside the carousel bounds and full scope/timer disposal. Shared overlays now
register their visible panel bounds with the existing focus manager. Local OCH-37
component acceptance is complete; navigation bit `274877906944` is advertised
(current aggregate `2199023255551`). Consolidated hosted checks and merge remain,
followed by final ticket completion. The chat showcase stays in OCH-46.
See [navigation design](design/navigation-components.md) and
[foundation evidence](evidence/navigation-components-och37.md).

OCH-38 now has a pure Core `Tree` collection with stable typed IDs, validated flat
forest topology, revisioned replacement, parent/ancestor/sibling metadata and
O(log n) payload updates sharing topology. Expect tests exercise malformed graphs,
100,000-node traversal/reorder, depth/metadata limits and distinct incarnation/
child revisions for future lazy-load admission. `Tree_state` now adds separate
incarnation-checked expansion/selection preferences, cached visible order, logical
cursor repair, single/multiple/range selection and pure tree keyboard reduction.
Tests cover hidden/disabled/reordered/reincarnated nodes, 100,000 selections and
application-payload collection. Core/Eio lazy loading now provides generation-
checked requests, 64 queued branches, four reusable workers, atomic child pages,
explicit retry and bounded error detail retention. Local runtime tests cover
cancellation without concurrency overshoot, queued-result reset, inbox backpressure,
shutdown and preservation of unrelated tasks. `Tree_rows` now projects item and
lazy-boundary records into keyed list data, with compact generation/incarnation-
checked identity, point invalidation and no historical key registry. Core tests
cover 100,000-node updates, 200,000 logical item/boundary rows, depth 128,
collapse/reopen and old-payload collection. The Bonsai tree-row primitive now
mounts only viewport/pinned rows, checks source-instance/reset identity, preserves
coalesced invalidation and retires old controller effects. Eio controls drive
capacity-limited visible demand, explicit retry and collapse cancellation; local
combined runtime tests distinguish view unmount from application data lifetime.
Tree metadata now reaches the native managed-list root and one focus-owning item
per row. Paired codec/Core/Bonsai tests and actual macOS AppKit checks cover
hierarchy, selection, expansion, disabled state, updates/removal and teardown.
The pinned macOS accessibility adapter adds reproducible disclosure getters.
A pure `Tree_interaction` reducer now checks source/node identity, preserves ordered
relative requests, separates activation, opens ancestors for logical reveal and
returns application-approved move proposals with approval-time revalidation.
Opt-in native keyboard/pointer requests now travel through monotonic list identity
to current Core/Bonsai handlers. Local macOS checks cover ordered arrows/modifiers,
AppKit focus/select, child editor/IME isolation and pointer priority. Explicit
reveal now hands focus to a stable row after asynchronous mounting, with bounded
pending state and cancellation on retirement, blur, deactivation or scrolling
away. Native tests cover a one-row budget and actual window activation changes.
Unicode typeahead now supports canonical accents, case folding, repeated-prefix
cycling and current-label search, with an event-driven native expiry clock and
bounded Core prefix. Codec, reducer, queued Bonsai and actual native key-dispatch
tests pass; a 100,000-node Core benchmark is recorded separately from native
workload acceptance. Per-row AppKit selection and expansion/disclosure setters now
carry explicit desired states, preserving ordered requests before rerender. The
public Bonsai widget now integrates preferences, ordered native/controller requests,
default row presentation and deferred reveal/focus. Its Eio filesystem example
passes actual directory loading, native focus retention, selection and stale-command
retirement after reset on macOS. Opt-in native dragging now emits typed move
proposals through current row identities, with native lifecycle cancellation and
Core/Bonsai endpoint validation. Local GPUI tests cover all placements, cancellation,
preview cleanup and embedded-editor isolation. The public editable outline passes
actual AppKit drag, confirmation and context-menu moves. Native GPU checks cover
actual focus/blur and all drop indicators above opaque rows, including changed
foreground and hover exit. Full traversal and revisit now pass separately through
Bonsai and native GPUI at 100,000 rows/depth 128, with 256 transient rows and weak
probes proving model/resource release. Native source reorder/window isolation,
deactivation/close, and public deep reveal, resize, loading/retry/cancellation and
window-scope cleanup now pass. Paint-time focus schedules one follow-up redraw to
publish its pin while OCaml is idle. Local OCH-38 component acceptance is complete;
managed-tree bit `549755813888` is advertised (aggregate `2199023255551`).
Consolidated hosted gates and merge remain before ticket closure. See the
[managed-tree design](design/managed-trees.md).

OCH-39 now has a tested Core column schema with stable keys, finite widths,
resize clamping, left pinning and keyed multi-level header groups. Tests cover
4,160 moves at the 64-column limit, invalid schemas and user restrictions. An
isolated styled DataTable candidate compiles against the unchanged GPUI pin after
a one-line macro crate-name fallback. Its native macOS probe renders at most
80 distinct body cells across four sampled positions in a 100,000-row/64-column
table. This is candidate evidence, not production integration or full-history
acceptance. Core data/paging and a scoped Eio adapter now pass 100,000-row order
and paged workloads, retired-row/query checks, saturated inbox delivery and
100 rapid resets while cancellation cleanup holds both worker slots. Producer
concurrency stays at two; old results are discarded and unrelated scoped work
survives explicit close. The selected native adapter now lives in `rust/table`,
with upstream provenance, existing base helpers and per-instance appearance.
Its macOS test reproduces the four virtualization samples, preserves selection
through row/column reorder, clears removed selections without event echoes, and
verifies native entity release on close. Native pointer tests now exercise keyed
double/context/sort/resize/reorder events and suppress obsolete frame input and
drags after schema refresh. Keyed pixel anchors survive native row/column reorder
and prepend; unchanged schemas preserve resize/reorder gestures through row
arrivals. The bridge, public widget, full paging/query reconciliation and native
keyboard/accessibility/cache/lifecycle acceptance remain.
Paired bounded table payload codecs and the pure Core `Table.Config` now pass
independent byte fixtures and invalid-input/budget tests. Transaction/event
envelopes now feed native tree admission, cell/schema byte accounting, ordered
command admission and live session input validation. Query resets retire viewport
handlers, and dirty cell edits revalidate table ownership. The retained native
host now renders real cell Views, publishes bounded viewport demand, executes
commands, shares focus retention and captures input routes at event creation.
A local background-window test passes a sparse 100,000-row source, row-height
anchor changes, single/empty data and entity release. The Core managed table View
now constructs bounded keyed cells, generates accepted schema revisions, routes
typed input through current query/schema/policy checks and emits ordered commands.
The public Bonsai presenter and Eio paging controls now connect bounded cell
lifetimes, membership-aware commands, query resets and current selection. Local
tests traverse all 100,000 rows twice and release retired cell payloads. The public
Table Lab passes native keyed scrolling/anchors, streaming, query retirement,
failure/retry and window cleanup. Native pointer/key dispatch and OS clipboard
checks now pass exact Unicode/quoted TSV, unavailable-selection preservation,
Tab exit, toolbar Copy, child-editor priority and hidden/disabled gating.
The public table now applies its style to one native root while separate
source identity preserves reset semantics. GPU checks pass alpha/gradient surfaces,
border/padding/corner clipping, inherited text, state precedence, pinned-column
paint and readable selected cells/rows. The styled Table Lab retains selection
and anchors through light/dark changes. Actual macOS accessibility now passes
logical counts/indices, Unicode cell values, selected-descendant focus, ordered
selection setters, separate sorting, selection-mode restrictions and retired
hidden/disabled objects. The vendored Cocoa adapter has a reproducible table
metadata/selection patch. The native table now passes two complete 100,000-row
traversals with at most 128 active rows/512 cells, actual horizontal sweeps and
zero retired text payloads retained at batch checks. This exposed a pinned Taffy
measurement-context retirement issue; a reproduced, documented two-line patch
at the same version fixes it in both backend paths. Unmount/window release and
intentional failure cleanup pass. The public example now also preserves anchors
and selection through accepted resize/reorder during a pending Eio page and
sorts while obsolete producer cleanup is held; late results do not alter the
new query. Shared controls/editor/list/tree/table regressions, Rust workspace
tests and strict Clippy pass. AppKit keyboard and embedded-editor composition now
pass through targeted OS events and the native text-input client. The public
example also passes pointer/keyboard inspection, guarded context actions, reveal,
focus restoration and native window closure. Selectable text now exposes its
content label to macOS accessibility. The [local acceptance audit](evidence/data-tables-och39-audit.md)
maps the live requirements to code and native evidence. Managed tables advertise
bit 40 (`1099511627776`), with shared mask `2199023255551`. Showcase integration,
hosted macOS/Linux gates and merge remain pending.
See [table design](design/data-tables.md) and
[evidence](evidence/data-tables-och39.md).

OCH-46 is now in progress. The chat's Explore workspace inspector integrates the
separate native counter package through its generated backend, with real property,
event and acknowledged-command flows. Local macOS pointer/keyboard, hide/reopen,
draft preservation and theme checks pass; both existing M4 chat acceptance suites
remain green. The native package's duplicate Space activation was fixed. Actual
light/dark screenshots were inspected and the controls refined. The inspector
also includes a lazily registered run diagram and bounded artifact history.
Native selection/keyboard movement, pointer drag, pan/zoom/reset, stage activation,
back/forward/replacement/breadcrumbs, preserved window state and themed screenshots
now pass locally. The new expect tests and existing review/M4 regressions remain
green. These are integrated flows, not completion of the full component matrix or simultaneous
workload. See [M5 integration evidence](evidence/agent-chat-m5.md).

The chat's source explorer also passes local macOS acceptance through the public
managed-tree/Eio APIs: lazy failure/retry, keyboard range/typeahead, drag and
context move approval/cancel, reveal, collapse cancellation, empty/reset and the
explicit 100,000-node fixture. The last-source viewport exposes 9 native AX rows
within its 24-row budget. Window-owned data/preferences survive inspector page
changes; fixture construction uses an Eio worker domain. Native theme tokens now
follow the chat palette, and all prior inspector/M4 acceptance suites remain green.
Actual source screenshots and scoped ownership evidence are recorded in the same
ledger.

The results inspector now passes its own local native walkthrough: full-query
sort/filter, paging/failure/retry/cancellation, cell/context/reveal/Unicode copy,
actual column resize/reorder, pin/reset and preferences across page unmounts.
The opt-in 100,000-row fixture exposes 7 AX rows / 32 cells at the last record,
within its 24-row / 96-cell budget. Shared fixture work is bounded to one running
producer and one latest replacement per window/fixture. Actual light/dark captures
led to corrected selection/hover contrast; GPU regressions verify both themes.
A held-pointer row-focus AX crash is fixed in the table adapter and protected by
a native intermediate-frame test. The full component matrix, combined workload,
consolidated hosted gates and merge remain pending.

The first settings pages now pass local macOS integration: a modal sheet with
window-owned stream chunk-size/pacing preferences, inclusive score-range filtering
of the actual results query, and an explicitly simulated six-digit connection.
Native numeric partial/invalid drafts, Return/Escape, three stepper presentations,
slider keyboard/AX edits and pointer preview cancellation, accepted-value remounts,
nested reset confirmation, OTP paste/clear, themes and composer preservation pass.
No native draft is replaced by an observation; closing discards uncommitted numeric
drafts and stale callbacks are fenced by the settings generation. Model tests
verify real fake-backend chunking/delays and score bounds. Existing results and M4
public/AppKit regressions pass. Date/color settings now also implement civil-date
review filtering/pagination, simulated confirmed follow-ups and concrete RGBA
annotations on the actual diagram. Local native keyboard/AX checks cover partial
and disabled dates, focus restoration, color validation/cancellation and retained
values across themes. Remaining presentation/navigation, motion/responsive and
combined-workload acceptance are still pending; see the M5 evidence ledger.

The review workspace also has local run feedback: ordered native rating requests,
retained private-note disclosure, single/multiple guidance accordion, and an
interactive contributor hover card linking to Sources. Local AppKit validation
covers keyboard/AX, pointer hover, focus restoration, note/rating retention across
route and inspector remounts, themes and composer preservation. Contributor initials
are shown initially; a local SVG portrait and actual unavailable-image decode/fallback
fixture can now be selected inside the preview. Existing
review extension and original M4 public/AppKit regressions remain passing.

The workspace tour uses a public native carousel of four attachment cards with
real destination actions. Local normal/reduced-motion runs verify keyboard and
control navigation, current semantics, focus/hover/hidden pauses and native
opt-in auto-advance. Contributor portraits reuse two window-scoped registrations;
actual native decoding failure shows initials, and switching back restores the
SVG image. The demo accepts `--reduced-motion` without changing OS preferences.
Remaining full M5 integration, combined workload and hosted/merge gates stay open.

The chat now has grouped public Sidebar destinations sharing inspector routes and
history, with independent branch expansion, SVG icon collapse and retained
offcanvas hiding/restoration. Local macOS pointer/keyboard/current-link and
light/dark walkthroughs pass; screenshots and source links are in the
[showcase evidence](evidence/agent-chat-m5.md). Public message/bubble/tool-result
cards, removable query tags, loading skeleton/shimmer/spinners, status and error
recovery now pass the Full/Reduce local presentation walkthrough, including exact
Unicode code/diff copy. Stage-context springs, ordered destination reveals and
shared response activity are integrated; local Full/Reduce geometry/interruption,
hidden context and response-input checks pass. The responsive inspector now passes
local Full/Reduce pointer/keyboard resizing, close/reopen geometry, Unicode draft
retention, hidden-alternative accessibility and independent-window checks at
1000–1360-pixel desktop widths. Native split tests also preserve marked IME text
and editor identity when the sibling pane closes. Combined workload and
hosted/merge acceptance stay open. Native documents now expose labelled groups
and dispatch toolbar/Markdown copy accessibility actions directly. The chat
regression verifies exact Unicode copy from a retained offscreen toolbar without
misrouting a click to another visible control, in Full and Reduce modes.

Streaming transcript jitter is also fixed: already installed documents no longer
insert/remove an Updating line for each pending parse, and initial preparation
does not paint a dummy source editor that disappears with the first Markdown
result. A failing native geometry
regression now passes in Flow/Viewport layouts, and the actual chat probe changed
from repeated 29-pixel rebounds to no downward steps while the composer stayed
fixed. See the [streaming evidence](evidence/agent-chat-m5.md#streaming-transcript-geometry-regression).

Milestone 05 is in progress on `milestone-5-ui-extensions`. OCH-23 static native
components pass local acceptance; OCH-24 has validated geometry, a bounded scene
codec, native resource ownership and a tested OCaml/Rust upload bridge. The
pure typed scene API, scoped Eio registration, bounded native geometry/job
preparation and GPUI worker/mesh painting are implemented. Hidden-window macOS
GPU tests pass for shapes, curves, clipping, pan/zoom, resizing and cleanup.
Native interaction state now covers selection, drag previews/cancellation,
position ownership, viewport policies and commands; GPU checks validate its
effective transforms through direct state calls. Bounded native text/raster/SVG
painting now passes GPU clipping, zoom, retired-source and deferred-work checks.
The typed canvas view, owner-aware reconciliation, bounded native tree admission
and revision-checked event bridge now have local regression coverage. Mounted
retained-tree GPU rendering passes publication, hide/show, command retention,
preparation failure/recovery, generation reset and repeated disposal checks.
Native-dispatch input checks now pass focus, drag/selection pixels, keyboard,
pan/zoom, wheel coalescing and cancellation, including actual window deactivation.
Native macOS object accessibility now passes label/selection/focus, separate
activation, transformed bounds, offscreen reveal, disabled/hide/removal checks.
The public OCaml Canvas Lab passes command/update/reset lifecycle checks with
184 and 19,024 items, plus external macOS AX/keyboard interaction checks.
The maximum mounted workload also passes with 20,000 marks, 2,048 interactive
objects, 4,096 accessibility nodes and zero retained accounting after each of
three update/resize/disposal cycles. Local canvas acceptance is complete and the
full canvas capability is advertised; consolidated hosted gates remain pending. See the
[extension](evidence/extensions-och23.md) and [canvas](evidence/canvas-och24.md)
evidence for exact completed scope. Its scope includes OCH-23–26 and OCH-33–39, followed by
[OCH-46](https://linear.app/ochat/issue/OCH-46/showcase-milestone-05-features-in-the-polished-agent-chat-reference):
showcase the completed feature families in the polished agent-chat application.
OCH-46 is part of milestone completion and covers feature mapping, interactive
flows, light/dark and responsive-layout polish, accessibility, reduced motion,
streaming/performance regression checks and updated screenshots/documentation.
Its [component coverage and flow plan](design/agent-chat-m5-showcase.md) is
versioned; its unimplemented rows and combined acceptance remain required work.
OCH-25 now has validated public spring parameters, an independent OCaml/Rust
parameter fixture and a tested analytic native spring trajectory. Typed programs,
a bounded codec and compiled finite sequence timelines also pass local tests.
The retained owner and bounded shared-clock registry now pass deterministic
lifetime, playback, phase and stale-paint tests. Atomic session admission and
compiled-storage quotas, GPUI rendering, and `View.animate_program` with bounded
stage-event batches are wired. Public Bonsai/Eio and expanded native macOS checks pass, including retained-list
and hidden-panel timing, deferred overlays, two-window clock lifetimes, spring
retarget controls and a 1,024-visible-owner workload with input and complete disposal.
The advanced-program capability is now advertised; hosted gates remain pending. See
[advanced animation evidence](evidence/animation-programs-och25.md) and
[design](design/animation-programs.md).
OCH-26 now connects typed container rules through retained views, bounded
selection events and native assigned-size layout. Local macOS checks pass resize
selection, keyed identity, nested/dialog/virtualized queries, hidden motion,
AppKit accessibility and IME isolation, pointer cancellation, fractional/scale
boundaries, observer/config lifetimes and a 256-query workload. The public
Bonsai/Eio example verifies retained editors and lifecycle behavior. Container
queries are advertised and final local capability checks pass. Consolidated
hosted macOS/Linux gates and merge remain pending.
See [container query design](design/container-queries.md) and
[acceptance evidence](evidence/container-queries-och26.md).
OCH-33 now implements bounded semantic metadata, native form associations and
stateless presentation/card helpers with public Core/Bonsai/Eio usage. Local
OCaml/Rust tests and macOS editor/AX/keyboard checks pass, including metadata
updates during IME composition and corrected light/dark card layouts. Avatar
image/fallback now passes actual GPU/AX, source lifetime, SVG resize recovery,
synthetic density and idle checks, with a public example covering ready/failure/
initials. Rating now provides controlled request reduction, native hover/stars,
keyboard and AX slider actions with read-only/disabled and modal/pointer policies.
Local acceptance passes, including 228 constrained-content combinations. The
presentation capability is enabled; consolidated hosted gates and merge remain. Native skeleton/shimmer/
spinner leaves now pass local reduced/static/ancestor-hidden idle, resume,
accessibility and disposal checks; the public example exercises all three.
See [presentation evidence](evidence/presentation-components-och33.md) and
the [Component Studio example](../examples/presentation/README.md).
OCH-34 has begun with validated shared numeric domains, min-anchored stepping,
and draft classification, with independent OCaml/Rust fixtures and boundary tests.
Slider contracts, bounded codec and native interaction state now pass local
checks. Retained slider views, tree admission and observation routing are implemented;
native single/range rendering and initial macOS pointer/keyboard/AX checks now pass.
Correlated commands and the public Bonsai/Eio slider controller/example pass local integration.
Slider foreground/focus styling now passes local GPU pixel checks for both axes,
light/dark palettes, display densities and constrained layouts. The public example
shows single/range and linear/logarithmic modes, all now exercised by external
macOS keyboard/AX automation. Decorated pointer geometry, capture loss,
minimize/restore and independent-window/close lifetimes also pass locally.
A three-cycle, 1,024-owner native workload passes bounded event/coalescing, idle
and owner-disposal checks; debug timing and batching limits are documented.
Number_input Core/wire contracts and bounded native codecs now cover draft/value
separation, historical domains, UTF-8 selection/IME ranges and guarded commands;
independent cross-language fixtures and semantic validation tests pass locally.
The native numeric policy model now validates commit/cancel/step, pending-edit
guards, composition/configuration preservation and revision/fault behavior with
deterministic state tests. Its callbacks now connect to one native InputState.
Retained numeric view descriptions, strict tree admission, owner/revision event
routing and byte-accounted change coalescing now pass local bridge tests.
Mounted numeric editing, basic step buttons, correlated commands and the Eio
controller are implemented. Initial macOS native checks pass keyboard commit/
cancel/step, clipboard, undo/redo, revision guards, retained configuration, marked
text composition and disposal. Native step-button hold-repeat now passes local
Sides/Stacked, cancellation, real window deactivation and idle-task disposal
checks. Actual AppKit numeric/text values, focus/edit/step/button actions in all
three stepper layouts, draft feedback, application metadata and IME/read-only/
disabled/hidden policies now pass locally. The public numeric example now passes
three-layout command/event, draft/history/selection, revision/lease/policy and
remount/close integration checks. Numeric GPU light/dark, density, focus and
constrained-layout checks now pass, with a stacked-button overflow fixed. External
macOS AX/OS keyboard tests also pass through the public example. Retained IME and
history through configuration changes, hidden/modal gates, independent-window
close during a held repeat, and three 256-editor workload/disposal cycles now pass.
OCH-34 local acceptance is complete for sliders, numeric editors/steppers and OTP.
OTP now has public Core/Bonsai/Eio controllers, bounded paired codecs, retained
native editing and correlated commands. Native key/clipboard/NSTextInputClient,
AppKit value/action/masking, public OS keyboard/AX, GPU light/dark/density/preedit,
managed-list pins, hidden/modal/capture cleanup and independent-window lifetimes
pass locally. Three 256-owner workloads verify bounded history/coalescing, idle
behavior and complete disposal. Capability `34359738368` advertises the family
(current aggregate `2199023255551`). Consolidated hosted gates and merge remain pending.
See [OTP contracts](design/otp-inputs.md).
See [numeric design](design/numeric-inputs.md) and
[foundation evidence](evidence/numeric-inputs-och34.md).
OCH-35 now has tested civil-date/month/selection/constraint models, typed locale
configuration, commands/observations, bounded paired codecs and a native calendar
policy owner. Partial ranges, historical selection validity, focus callback
failure and revision exhaustion have model coverage. Gregorian-cycle, daily
reference, malformed-wire and maximum-config checks pass locally. Retained calendar
views, bounded tree admission, seed/history semantics, revision-checked event routes
and atomic completion mailbox admission now pass paired-codec/Core/native tests.
The mounted GPUI calendar now passes initial macOS day/month/year keyboard/pointer,
single/range event ordering, retained historical state, hidden/read-only/disabled
focus, pair-overload and disposal checks. Four initial light/dark GPU readbacks were
visually reviewed. Public correlated controllers now pass local real-window checks
for commands, revision/lease guards, admission limits, policy changes and close
ordering. Hidden focus cleanup now reports actual platform focus. A public popup
picker now separates application values from native drafts, with explicit
Apply/Cancel, revision/session guards and external-value invalidation. Local
macOS AX/OS-input checks cover popup selection, dismissal and focus behavior.
Inline AppKit values/cursor/selection and OS keyboard/Tab navigation also pass,
with native civil endpoints, leap clamping and locale retention checks. Calendar
rendering now passes 192 GPU theme/density/font/constrained-layout cases; long
labels use ellipses and caller font overrides are honored. Native modal/pointer
gates, managed-row pins, independent windows and three 64-owner idle/disposal
cycles also pass locally. Single/range popup mode changes, nested-dialog dismissal/
focus, actual panel bounds and right-edge placement also pass. OCH-35 local
acceptance is complete; capability `68719476736` is advertised (aggregate
`2199023255551`). Consolidated hosted gates and merge remain pending.
See [calendar design](design/calendar.md) and [model evidence](evidence/calendar-och35.md).
OCH-36 now has concrete Core/Rust RGBA/HSLA conversion and bounded hex-draft
models, separate from theme references. The pure native policy now tests
preview/commit/cancel, draft/composition handling, hue memory, guarded Set/Reset,
stale callbacks, configuration history and revision/fault lifetimes. Core control
contracts and bounded standalone Rust/OCaml codecs now pass independent byte
fixtures and malformed/maximum-payload checks. Retained descriptions, tree seed/
history semantics, revision-checked routing and atomic bounded event batches now
pass local bridge tests. Initial mounted channel/palette checks now pass native
keyboard/Tab, pointer preview/commit/Escape, configuration-during-drag, disabled
isolation and disposal. Native Hex/HSLA text fields now pass draft/commit/cancel,
composition, history/configuration preservation, bounded storage and child
disposal checks, including actual AppKit marked-text/insertion delegates. Runtime
controllers and a public Color Studio example now pass real-window correlated
command, revision/lease, policy, request-limit, remount and close checks. Popup
selection now passes public session/policy/Apply/Cancel integration and actual
macOS AX/OS keyboard, dismissal, focus restoration, nested-dialog and clamped
placement checks. OCH-36 local acceptance is complete: GPU checks cover 64
light/dark/density/font/constrained-layout cases and transparent/opaque/empty
swatches; native tests cover composition/managed-row pins, hidden/modal/pointer
gates, independent-window deactivation/close and three 64-owner/320-editor
workload/disposal cycles. A short-height clipping bug was fixed and channel
fields now use available width without rounding native values. Color capability
`137438953472` is advertised (aggregate `2199023255551`). Consolidated hosted
macOS/Linux gates and merge remain pending. See [color design](design/color-inputs.md)
and [foundation evidence](evidence/color-inputs-och36.md).
No milestone-05 completion or hosted acceptance is claimed yet.

The [milestone acceptance ledger](evidence/milestones-01-02.md) maps every ticket
to its implementation and evidence. The first consolidated hosted run passed all
macOS checks and Linux build/unit tests; one Linux-only native-test lint issue was
corrected for final-head verification. Linux GUI remains informational. See
[theme/scale review](evidence/theme-scale-och11.md) and
[animation contracts](design/animations.md). Checkpoints below describe the state
at the time and preserve earlier validation findings.

Repository: `dakotamurphyucf/gpuio`, public, Apache-2.0, default branch `main`.
These settings were selected by the owner on 2026-09-11.

Platform priority updated by the owner on 2026-09-11: macOS is the primary
functional acceptance platform during implementation. Linux builds/unit tests
remain required, but graphical checks are informational and full Linux GUI
validation is deferred to OCH-17. Linux remains an intended platform. Earlier
design documents requiring native GUI acceptance on both OSes before advancing
are superseded by this priority; native GUI coverage must still be reported honestly.

- OCH-18 complete: remote scaffold, standards/design import and fresh-clone checks.
- OCH-19 complete: pinned OCaml/Rust closure, reconstructed
  native Bonsai sources and patches, codec/lifecycle checks.
- OCH-20 complete: isolated bootstrap and contributor tools.
- OCH-21 complete: source-built Dune/Cargo smoke app and
  two-window native identity/lifetime scenario.
- OCH-22 complete: required builds/tests, macOS native checks, informational Linux
  graphical checks, retained evidence and protected main branch.
- OCH-6 setup gate complete, merged in PR #1 at `81f6b581c784d448a8948d4cb55e73af9db4b86c`.
- OCH-7 complete, merged in PR #2 at `e6471b4ec88e6847f950da30576b6d2a6639d930`.
  CI run34650637422 passed on both OSes, including production 50-revision/
  two-window/rollback/panic smoke on macOS, X11 and Wayland.
- OCH-8 complete, merged in PR #3 at `8f7fd9f357a0b8df3e9dfe31c2a7217925c8846d`: typed views/styles/themes, keyed reconciliation, pure Bonsai
  adapter, native button/selection behavior and GPUIX style mapping. Local macOS
  tests and both required CI jobs passed in run 34654290650. X11 passed all GUI
  checks; Wayland passed the typed bridge but failed the new hover-reset test.
  That informational limitation remains tracked in OCH-17.
- OCH-9 public Bonsai/Eio runtime merged in PR #4 at
  `02558d8d393c49e5e159812394dd9061820c39fc`. Both required CI jobs passed in
  run 34740866262, including all macOS runtime/measurement scenarios. Linux GUI
  exposed a default quit-policy difference, fixed in PR #5 at
  `88cc9287db49cd27c0b78a6f19eea17fc4c1069e`. Final run 34741216419 passed both
  required jobs and all OCH-9 scenarios on macOS, X11 and Wayland. X11 passed
  the full GUI suite; the existing Wayland hover-reset issue remains under OCH-17. See [runtime](design/runtime.md) and [measurements](evidence/runtime-och9.md).

## Local evidence

macOS arm64, stock OCaml 5.3.0, Dune 3.24.2, Rust 1.97.1. The separate
`.opam-root/gpuio` was created from the pinned opam repository; the existing Ochat
switch and default toolchain selections were not modified.

- Core/PPX expect tests and native Bonsai lifecycle tests pass, including
  optimized/unoptimized graphs, unchanged views, keyed retention, cleanup and a
  dedicated OCaml domain.
- The OCaml and Rust codec checks independently construct, encode and decode the
  same 96-byte fixture with full byte consumption.
- The actual GPUI window self-test passes: 50 checked commits, 1525 command bytes,
  native input-handler probes, stale event rejection, Rust panic containment,
  balanced 23/23 row activation/deactivation, Eio cancellation and clean shutdown.
- Two actual windows pass distinct identity, independent editor state, stale
  window rejection and continued use of the surviving window after closing the
  first. Both close and return through the FFI.
- First-party Rust passes Clippy with warnings denied. GPUI's transitive `block`
  0.1.6 reports a future-compatibility notice; it does not fail the pinned build.

`docs/evidence/macos-arm64-packages.txt` is the actual isolated package inventory.
Historical research documentation is preserved under `docs/design` and is not
an assertion of current production API functionality.

## Hosted evidence and remaining platform validation

PR run [34646959232](https://github.com/dakotamurphyucf/gpuio/actions/runs/34646959232)
passed on macOS ARM64 and Ubuntu 24.04 x86-64. The informational Linux GUI report
also records X11 and Wayland success: both asserted the intended backend and
passed the 50-commit native/lifecycle/input-handler example and two-window
identity/cleanup scenario. X11 used Xvfb/Openbox; Wayland used nested Weston;
Mesa software Vulkan supplied rendering. This is actual backend window coverage,
distinct from the earlier accidental headless X11 attempt. Both pinned language servers have passed hover and
go-to-definition checks; evidence is in `docs/evidence/*-lsp-navigation.json`.
Main requires PRs and both `foundation (macos-15)` and `foundation (ubuntu-24.04)`
checks with an up-to-date branch. Force pushes and branch deletion are disabled.
Linux GUI outcomes remain informational and do not alter this development gate.
No full OS IME automation, accessibility or production multi-window Bonsai API is
claimed by the bootstrap smoke tests. Those remain in their owning v1 tickets.

## Typed API validation (OCH-8)

The pure API tests cover callback-only refresh, keyed reorder/replacement, invalid
plans, theme changes, style composition/reset and bounded incremental output.
OCaml and Rust independently agree on every expanded style tag in `style-v1.hex`.
Native tests validate malformed styles, rollback and nested memory accounting.
The actual macOS window test passes grid bounds, hover/pressed/focus, Enter/Space,
Tab/Shift-Tab, pointer policy, Unicode select/copy, replacement and inherited reset.
The public OCaml example passes 20 acknowledged native commits and theme changes.
The [typed API contract](design/typed-ui.md) records all GPUIX style mappings and
functional limits. Linux graphical execution remains informational under OCH-17.

## Milestone 02

OCH-10 implements native input/composer ownership, stable Bonsai/Eio controllers,
revisioned commands, native composition and grapheme editing, undo/redo selection,
auto-grow and basic accessibility. [PR #6](https://github.com/dakotamurphyucf/gpuio/pull/6)
and [its evidence report](evidence/native-editor-och10.md) record implementation
and platform validation. Hosted run 34745383026 passed Linux build/tests/lint and
macOS editor/input/accessibility checks. X11 passed the complete GUI suite;
Wayland passed public editor commands but its clipboard-based native test failed
before insertion, tracked in OCH-17. These checks do not claim physical IME
candidate-panel or complete screen-reader coverage.

Current OCH-11 summary: the controls/commands/focus/overlays, pointer capture,
file-dialog bridge and drag/drop behaviors below are implemented and validated
locally. Drag/drop includes actual AppKit handoff/reentry/cancel/unmount and held-
gesture close/shutdown checks. Raster/SVG assets, bounded caches and foreground-tinted icons now pass local
integration checks. Decorative button/command-button icon slots also pass local
checks. Nested transcript/code/composer/popup/modal scrolling and scroll-owner
disposal also pass local native checks. Remaining work includes theme/scale and
native-state audit, basic transitions and aggregate lifetime review. Consolidated macOS/Linux CI and merge
remain. The chronological checkpoints below distinguish earlier partial states
from later validation; they do not all describe the latest remaining scope.

OCH-11 is in progress: controlled checkboxes/switches and disabled buttons merged
in PR #7 (`0ef2c7dc5b71235c090d4dc6373f505db69624e5`); CI 34747606484 passed both
required jobs and control windows on macOS, X11 and Wayland. The existing Wayland
editor clipboard limitation remains under OCH-17. Radio groups in PR #8 pass
both required jobs in CI34748629291, including actual control windows on macOS,
X11 and Wayland. PR #8 merged as `5448842ffd9bff9e249071698a294a3afc3ffb42`.
Select PR #9 merged as `8a9167cf227a290f967452c051f0b4c7dde19461` after both required
jobs in CI34750240273 passed. Its adapter adds native popup navigation/cancellation, current-frame
positioning and virtualized options. Local macOS window/accessibility checks,
4096-option navigation, OCaml/Rust tests, full build/format and Clippy passed.
Choice appearance adds theme-aware popup/option/empty styles, configurable uniform
row geometry and localized empty text while retaining native focus/open state.
Local tests validate these changes; general overlay integration remains OCH-11 work. [Native controls](design/native-controls.md) records these families
and the remaining ticket scope. OCH-12's declarative animation configuration and
timing core and view/bridge/native integration now pass local tests; application
motion policy and platform preference detection remain pending.
Combobox is implemented on the local OCH-11 branch with native editor ownership,
query filtering, exact selection snapshots, the editable accessibility role and
shared popup appearance/virtualization. Local native control tests pass including
macOS marked/committed text. The public controller smoke passes conditional
replacement, stale revisions, undo and unmount; existing two-window editor commands
also pass after sharing the controller implementation. OCaml/Rust tests and Clippy
pass locally. Full build/format validation is recorded with the local change.
Per owner instruction, remaining OCH-11 work stays local until the complete ticket
is ready for a consolidated CI pass. No Combobox hosted acceptance is claimed.
The broader component catalog is planned in OCH-33–45; vendoring GPUI Base does
not expose all of those widgets through the OCaml API.

Focus scopes are also implemented locally for OCH-11: native Tab trapping, nested
entry/restoration, hidden/disabled traversal, empty-root fallback, command and
accessibility gating, and bounded cleanup pass actual macOS control-window tests.
The other remaining OCH-11 families are still in progress.
No hosted acceptance is claimed for this local scope implementation.

Dialog/Popover surfaces are implemented locally with application-controlled
content lifetime, typed dismissal, native stacking/placement and accessibility
semantics. Local macOS tests pass nested dialogs, restoration, choice-popup
interaction beyond panel bounds, marked-text Escape and moving anchors. The
public Bonsai/Eio overlay example passes native mount, editor commands, modal
focus denial, stale unmount and close. OCaml/Rust tests, independent fixtures,
full build/format and Clippy pass locally. No hosted acceptance is claimed;
tooltips/menus/commands and the rest of OCH-11 remain in progress.

Anchored placement is implemented locally: popovers accept preferred side,
start/center/end alignment and signed offset, with current-frame edge flipping
and viewport clamping. Local native checks retain focus while changing placement
and moving the anchor; positioning/validation unit tests and independent protocol
fixtures pass. The extension appends a new operation without changing earlier
overlay records.

Tooltips are implemented locally with managed or application-controlled visibility,
retained arbitrary content, delayed hover, shared grace timing and keyboard
opening/dismissal. Hidden content preserves native editor identity while denying
focus and deactivating nested traps. Local macOS tests pass hover cancellation,
interactive content, tooltip/popover hit routing, accessibility exposure and
bounded timer/subscription disposal. The public Bonsai/Eio example passes retained
editor commands, controlled visibility, stale unmount and shutdown. Independent
protocol fixtures, OCaml/Rust tests, full build/format and Clippy pass locally.
No hosted or full Linux GUI acceptance is claimed for this local checkpoint;
menus/commands, feedback, pointer/desktop interactions and assets remain OCH-11 work.

Shared command registries, command buttons and single-chord shortcuts are
implemented locally. Native macOS tests pass scoped dispatch, native editing targets,
keyboard/IME priority and two-window isolation, including closing one window and
continuing in the other. Independent OCaml/Rust protocol fixtures pass. The public
Bonsai/Eio example, full OCaml build/tests/format, Rust workspace tests and
Clippy pass locally. Menu adapters are being validated locally as described below;
the command palette and other OCH-11 requirements remain In Progress.


Menus are implemented on the local OCH-11 branch: immutable command-reference
models, dropdown/context/in-window/platform presentations, virtualized cascading
popups and active-window macOS menu ownership. Targeted macOS native tests pass
actual NSMenu and accessibility activation, right-click Copy/focus restoration,
1000-entry wheel/keyboard navigation, popover integration, focused command scopes,
hidden/stale actions and menu restoration after closing a second window. The
activation test found and fixed menu ownership refresh when returning to an
unchanged surviving window. Hidden triggers now close detached popup state and
release focus. The combined native controls suite, public Bonsai/Eio example,
Rust workspace tests and Clippy pass locally; the [menu evidence report](evidence/native-menus-och11.md)
records coverage and limitations. No hosted or Linux GUI acceptance is claimed.


The command palette is implemented on the local OCH-11 branch: ordered command
references, native query/composition/history, virtualized results, shared command
execution, modal focus and accessible activation. Local macOS tests pass a
1000-command list, current-query/current-generation routing, native document Copy,
hidden/nested-modal restoration and query disposal. Visibility-driven dismissal
now runs before paint can discard focus ancestry. Full OCaml build/tests/format,
Rust workspace tests, Clippy and the public Bonsai/Eio example pass locally. The
[palette evidence report](evidence/native-palette-och11.md) records the checks and
an unresolved intermittent tooltip-hover failure seen in an earlier combined run;
the subsequent combined controls run passed. OCH-11 remains In Progress, with no
hosted or Linux GUI acceptance claimed for this checkpoint.


Progress indicators are implemented locally for OCH-11 with validated fractions,
explicit indeterminate state, native animation, percentage accessibility and the
existing root-style/theme API. Actual macOS tests pass painted dimensions/colors,
noninteractive focus, animation without OCaml commits, and hidden/determinate/
unmount cleanup. The public Bonsai/Eio example, combined controls suite, full
OCaml build/tests/format, Rust workspace tests and Clippy pass locally. The
[progress evidence report](evidence/native-progress-och11.md) records the scope;
in-app notifications are described below and OCH-12 still owns general motion
and reduced-motion integration. No hosted or Linux GUI acceptance is claimed.


In-app notifications are implemented on the local OCH-11 branch with keyed
terminal sessions, bounded stacks, explicit overflow, native active-time deadlines
and hover/focus/hidden/modal pause. Local macOS tests pass close/accessibility/
keyboard actions, native editor Escape priority, ordinary action content, expiry
without OCaml commits and unmount cancellation. The public Bonsai/Eio example
passes native dismissal delivery and keyed removal. Independent protocol fixtures,
OCaml expect tests, Rust workspace tests and the combined native controls suite
pass locally. The [notification evidence report](evidence/native-toasts-och11.md)
records exact coverage and remaining validation. No hosted or Linux GUI acceptance
is claimed. OCH-11 still includes pointer capture/drag-drop/file dialogs, assets/
images/SVG/cache and theme-scale integration, remaining state/transition work,
scrolling/lifetime checks, documentation and consolidated CI/merge.

Captured pointer regions are implemented locally for OCH-11. Real macOS native
tests pass out-of-bounds movement, redraw/reposition retention, cancellation,
modal gating, nested ownership, native child-control precedence and pressed
styling. The combined controls suite passes after final release-order review;
Clippy and the full Dune build/tests/format also pass. Independent protocol/Core
tests cover validation, callback lifetimes and motion coalescing. The public
Bonsai/Eio resize example passes its lifecycle self-test. See the
[pointer evidence report](evidence/native-pointer-och11.md) for precise coverage.
Pointer capture remains distinct from drag/drop and file dialogs, which are still
pending alongside assets/images/SVG/cache, theme-scale integration, remaining
state/transitions, scrolling/lifetime checks and consolidated CI/merge. No hosted
or Linux GUI acceptance is claimed for this checkpoint.

File-dialog implementation has started with pure OCaml/Rust path and open/save
configuration models. Focused Core expect tests, Rust protocol tests and Clippy
pass, including exact non-UTF-8 path bytes, filename validation and selection
limits. These constructors do not present dialogs. The
[file-dialog design](design/file-dialogs.md) records the
contracts, pinned-source findings and remaining acceptance work.

The macOS Rust file-panel adapter now passes native sheet presentation, file and
directory selection, exact save-path return without file creation, Busy,
cancellation and owner disposal checks. Full Clippy, Rust workspace tests and
Dune build/tests/format pass with its direct macOS dependencies. The
[file-panel evidence](evidence/native-file-dialogs-och11.md) describes the actual
AX-based test and its permission requirement. The bridge checkpoint below adds Runtime/Eio/Bonsai integration and application
close cancellation. Capability reporting and Linux portal support remain pending;
this is not a completed file-dialog feature or OCH-11 ticket.


The OCH-11 file-dialog bridge now connects the OCaml configuration models to
window-owned macOS panels through correlated Bonsai/Eio effects. Local native
ownership tests and public close/shutdown tests pass; an end-to-end test selects
the LICENSE file through real AppKit controls and reads it explicitly with Eio.
Independent fixtures cover exact raw path bytes; result decoding and mailbox
accounting enforce count/size bounds. See the updated
[file-dialog evidence](evidence/native-file-dialogs-och11.md). Capability queries
and the Linux portal backend remain pending (non-macOS currently returns
Unsupported), so file dialogs and OCH-11 are not complete. No hosted CI or Linux
GUI acceptance is claimed for this checkpoint.

The Linux file-dialog protocol layer is now implemented in the new `gpuio-portal`
workspace crate. Fourteen local D-Bus socket-peer tests pass for request/reply
races, cancellation/cleanup, service identity/loss and bounded URI results. It
reuses existing locked dependency versions. The crate is not yet connected to
the native runtime: X11/Wayland parenting, cleanup barriers, capabilities and Linux
validation remain pending. See the [portal design](design/linux-file-portal.md).
No actual Linux portal GUI or completed OCH-11 support is claimed.

The next local checkpoint connects the portal worker to X11 native requests.
Window-close/shutdown cleanup now waits for background workers, including a
response already being delivered. Quit cleanup runs before GPUI clears windows;
ordinary lifecycle cleanup stays asynchronous. The shared ownership adapter's
tests, full Rust/OCaml checks, actual macOS picker suite and public Bonsai/Eio
close/selection/read regressions pass. See the updated
[portal evidence](evidence/linux-file-portal-och11.md). Wayland exports, public
capabilities and Linux build validation remain pending. OCH-11 stays In Progress.

Wayland file-dialog parenting is now implemented locally with one shared guest
registry per application display and separately owned surface exports. It uses
GPUI's existing socket reader, bounded pending-export polling, cancellation and
the native cleanup barrier. Full workspace Clippy/Rust and Dune checks pass on
macOS; three new system-libwayland protocol tests compile but are explicitly
ignored here and await Linux execution. Public capabilities and consolidated
Linux/macOS CI remain pending. No Linux GUI or complete OCH-11 acceptance is
claimed; see the [Wayland checkpoint evidence](evidence/linux-file-portal-och11.md).

Public file-dialog capabilities are now implemented locally: a per-window typed
snapshot reports single/multiple selection by mode and save support, with the
same Not_ready/Busy/Closed lifecycle as pickers and no picker presentation.
Local macOS native/public tests, independent OCaml/Rust fixtures, portal version/
no-presentation tests, full build/format and Clippy pass. Existing real selection
and Eio-read regressions pass after sharing the correlated query path. See the
[file-dialog capability evidence](evidence/native-file-dialogs-och11.md). Linux
build/unit verification (including three ignored-on-macOS Wayland tests), remaining
OCH-11 feature families, consolidated CI and merge are still required.

OCH-11 drag/drop now has validated OCaml/Rust payload, source and target models,
plus bounded bin_prot codecs and independent byte fixtures. Text, raw Unix paths
and opaque custom data retain distinct validation rules; desktop-file offering
requires explicit directory metadata and native target acceptance uses an exact
format allowlist. Full local workspace Clippy/Rust and Dune build/tests/format
pass. These are data/configuration tests, with no native drag/drop interaction
claimed yet. View/event/native ownership integration is next; see the
[drag/drop design and remaining acceptance](design/drag-and-drop.md).

The next OCH-11 drag/drop checkpoint integrates source/target views through
reconciliation, protocol, native trees and Bonsai/Eio event routing. Local macOS
native window-dispatch tests pass for nested acceptance, immutable gesture
snapshots, cancellation/removal, raw incoming files, size limits and release of
source/hover state. The public example's lifecycle test, independent operation/
event fixtures, queue/ownership tests, full builds/format/Clippy and existing
native pointer regressions pass. The bridge advertises drag/drop bit 1048576
(required mask 2097151). See [integration evidence](evidence/drag-drop-och11.md).
Actual OS file export/reentry, multi-window/focus/active-close checks and public
gesture callback testing remain; this is not completed drag/drop or OCH-11
acceptance. No hosted CI or Linux GUI acceptance is claimed.

Actual AppKit mouse dragging now passes through the public Bonsai/Eio example:
matching gesture identity/payload, accepted hover, result update, painted frame
and clean shutdown. Expanded native checks pass focus-trap cancellation and
second-window activation/recovery with late-release suppression. No production
runtime patch was needed for the system-event test driver. Native Clippy, full
Dune checks and the ordinary lifecycle example pass. See the updated
[drag/drop evidence](evidence/drag-drop-och11.md). OS file export/reentry, live-close/
shutdown and remaining OCH-11 families still require work; no hosted CI is claimed.

Actual macOS file-session checks now pass through the public Bonsai/Eio API:
second-window Desktop delivery with a distinct gesture ID and unknown metadata,
source-window reentry restoring original identity/metadata, OS Escape without a
drop, and source unmount suppressing late callbacks while the immutable OS offer
remains receivable. The temporary source file remains unchanged. These are real
AppKit sessions between child windows, not Finder/external-copy acknowledgement
or Linux GUI coverage. See [drag/drop evidence](evidence/drag-drop-och11.md).
Live-window close/shutdown while dragging and the aggregate lifetime review remain,
as do the other OCH-11 families and consolidated CI/merge.

Held-gesture close/shutdown validation now passes locally for both internal drags
and OS-owned file sessions. AX confirms physical source-window removal; a surviving
window paints after an explicit mouse-release handshake. App shutdown returns
cleanly with no callbacks to disposed sources. The drag-specific ownership review
found no reference cycle and records bounded snapshots/hover state separately from
OS payload lifetime. Native Clippy, full Dune checks and transfer/unmount regressions
pass. See [drag/drop evidence](evidence/drag-drop-och11.md). Remaining OCH-11 feature
families and consolidated platform gates are unchanged; nothing has been pushed.

Asset work has started with immutable OCaml/Rust source descriptors for the nine
pinned GPUI format families. Constructors preserve opaque encoded bytes, enforce
nonempty/16-MiB bounds and report format/length rather than dumping contents.
Core expect tests, targeted Rust tests, protocol Clippy and full Dune build/tests/
format pass. The [asset design](design/assets.md) records the required chunked
transport under the existing 1-MiB envelope and the native ownership/cache plan.
Registration, decoding and image/icon views are not implemented by this checkpoint;
no asset capability is advertised yet.

The native application session now owns a bounded encoded asset registry: ordered
chunk staging, complete-data publication, generational IDs, retirement and terminal
shutdown. Existing readers keep retired data valid and charged until they release
it; retired handles cannot create new uses. Five registry tests and the session
lifecycle test pass, along with the Rust workspace, native Clippy and full Dune
checks. See [asset evidence](evidence/assets-och11.md). This registry is not yet
connected to FFI upload commands or the OCaml runtime, and no image/icon rendering
or new capability is claimed. Those integrations are the next OCH-11 work.

Encoded assets now cross the FFI using bounded correlated Begin/Append/Finish/
Release messages and reserved responses. Independent OCaml/Rust fixtures,
mailbox-pressure tests, full Rust/Clippy and Dune checks pass locally. A windowless
public Eio runtime example uploads >2 MiB and verifies release, stale IDs, invalid
uploads, quota recovery and shutdown. The new capability 2097152 (aggregate
4194303) advertises encoded registration only. Scoped public ownership, decoding,
image/icon views and cache cleanup remain; see [asset evidence](evidence/assets-och11.md).

Scoped encoded registration now uses `Gpuio_eio.Asset.register app ~scope source`.
The adapter bounds queued source bytes/live metadata, suppresses cancelled user
completions while accounting for late allocation replies, and reserves one request
lane for upload/cleanup independent of raw traffic. The windowless native example
passes public registration and scope retirement under saturated raw request lanes,
with subsequent full-quota allocation proving reclamation. Deterministic scope tests
exercise every upload cancellation boundary. Decoding and pure image/icon views are
still pending; this is encoded ownership, not rendered-image acceptance.

The native in-memory raster decoder now covers PNG/JPEG/WebP/GIF/BMP/TIFF/ICO/PNM,
GPUI BGRA ordering, static EXIF orientation, GIF delays and complete-result failure
on malformed frames. It checks dimensions and retained pixel/frame bounds. This
helper is not yet scheduled from the host or exposed in views; SVG, aggregate
worker/cache ownership and actual rendered-image acceptance remain pending.
See [asset design](design/assets.md) for strict-output versus best-effort decoder
allocation limits and [pixel-test evidence](evidence/assets-och11.md).

The decoded-cache/work-ticket controller now reserves result output before native
work dispatch, bounds live/queued/running/retired state, shares source decodes and
keeps evicted pixels charged through their last reader. Worker/handle identities
reject late or foreign results; mounted-owner disposal directly cancels work.
Controller tests include an actual background-thread decode. The host does not
yet schedule these tickets or perform per-window atlas evictions; SVG and image
views are still pending. See [asset ownership design](design/assets.md).

The production native host now initializes the image scheduler, launches admitted
decodes on GPUI's background executor, refreshes windows, accounts for per-window
image uploads and drains workers/atlas cleanup during shutdown. A local macOS test
with focus disabled passes exact two-window GPU readback and verifies eviction by
forcing a same-ID diagnostic reupload with different pixels. It also passes close,
replacement and two-outstanding-job shutdown checks. The optional native-image-tests
feature/CI target adds test-only readback support; no hosted run or Linux GPU result
is claimed. Public OCaml image/icon views and SVG remain pending; see the
[asset evidence](evidence/assets-och11.md).

Declarative raster image views now work through the public scoped Asset/Bonsai/Eio
path, with immutable application-specific handles, fit/description configuration,
loading/ready/failure observations and native mounted leases. Pure owner/protocol/
reconciliation tests, native tree validation, full Dune/Rust workspace checks and
feature-enabled Clippy pass locally. A background macOS production-view test passes
exact GPU pixels, retirement/restyle/replacement/disposal and AXImage label checks;
the public example separately passes actual FFI event integration. See
[asset design](design/assets.md) and [asset evidence](evidence/assets-och11.md).
SVG/icons and the remaining OCH-11 families are still pending. CI definitions are
updated, but hosted/Linux gates and merge remain deferred until local scope is done.


SVG/icon rendering now passes local native GPU and public OCaml tests. SVG views
preserve color; icons tint the alpha mask with inherited foreground. Native resize
and hover select new size/density/fit/tint variants without an OCaml transaction,
including after registration retirement. The background production-view test passes
actual color/resize/tint pixels and immediate weak-binding cleanup; it uses synthetic
GPUI hover dispatch and does not claim physical monitor-scale changes. Decoder/cache
and cross-language Icon fixtures, full Rust workspace and Dune checks pass. Public
raster/SVG/icon example modes all pass lifecycle and FFI state integration. See
[SVG evidence](evidence/assets-och11.md#svgicon-integration--local-macos-continuation).
Remaining image clipping/composition and the other OCH-11 acceptance/gates remain.


Image corner propagation now passes an actual native regression: asymmetric raster
corners and changing icon hover radii clip the pixels while preserving center color,
image lifetime and AXImage semantics. See [clipping evidence](evidence/assets-och11.md#native-image-corner-clipping).
Icon/control composition and the remaining OCH-11 acceptance/gates are still pending.


Decorative leading/trailing button icons and labelled icon-only buttons now compose
with existing native activation/focus/accessibility. Core identity tests, native atomic
slot validation, actual GPU/AXButton and synthetic GPUI input/command-label checks pass.
Public raster/SVG/icon example modes pass with the new button compositions. See
[button icon evidence](evidence/button-icons-och11.md). OCH-11 remains in progress;
remaining theme/state/transitions, scrolling/lifetimes and consolidated gates remain
at that checkpoint.

Nested container scrolling now passes local native acceptance for transcript,
horizontal code, composer, Select popup and modal shielding. Same-node offsets
survive updates, wheel input leaves the tree revision unchanged, and removed
scroll owners dispose immediately. Native image/button and pointer regressions pass;
see [scrolling evidence](evidence/scrolling-och11.md). OCH-11 still needs the remaining
theme/state audit, basic transitions shared with OCH-12, aggregate lifetime review
and consolidated macOS/Linux gates and merge.

Native command routes now share immutable registry entries instead of cloning
label/shortcut payloads per button/menu/palette route. A 1,024-route lifetime test
checks sharing, stale-generation rejection and final-owner disposal; the full
native controls and image/button suites pass locally. See
[command lifetime evidence](evidence/command-lifetimes-och11.md). This closes the
identified command-payload duplication concern; remaining OCH-11 scope and hosted
gates are still pending.

OCH-12 now has validated OCaml animation configuration and a deterministic Rust
timing core. Tests cover delayed starts, paint-confirmed completion, interruption,
repetition, hidden/reduced-motion state and prepared-frame invalidation. The numeric
configuration has an independent OCaml/Rust binary fixture. No animation capability
is advertised: View/reconciliation/transport, GPUI scheduling, platform motion
preferences and actual native acceptance still need implementation. See
[animation design and current evidence](design/animations.md).

The OCH-12 view/bridge/rendering pipeline now passes local native and public checks.
`View.animate` retains node/run identity, delivers typed endpoints, and applies
Rust-computed values to GPUI. Actual native tests cover sidebar geometry without
inner reflow, interruption, native repetition, whole-window idle/hidden/reduced
behavior and delayed-task disposal. The public Bonsai/Eio example passes endpoints,
theme change and shutdown. Platform preference detection, application policy,
final capability advertisement and consolidated gates/merge remain pending; see
[animation design](design/animations.md).

Shared motion preferences now work through `App.run ~motion` and `App.set_motion`.
macOS uses a live NSWorkspace observer; Linux has an event-driven XDG Settings
adapter with bounded calls and documented unavailable-setting fallback. The native
macOS animation suite passes policy changes and a real notification/disposal check;
the public example passes immediate settling of a long animation under Reduce.
Portal protocol tests pass using a private mock connection on macOS. Final
capability/acceptance, hosted macOS/Linux gates and merge remain pending; see
[animation policy](design/animations.md#application-motion-preferences).

Consolidated local milestone 02 acceptance (2026-09-24): full Rust workspace,
Dune `@all @runtest @fmt`, and all-target feature-enabled Clippy pass. Native
animation, controls, progress, image/scale, drag/drop and AppKit file-dialog suites
pass. Public animation, drag/drop and file-dialog lifecycle examples pass with the
final capability mask. The animation test now activates its window: controlled
activation proved that a fully occluded background window was waiting for its
first frame. This change affects test reliability, not production window policy.
Hosted macOS/Linux validation and merge remain pending.
