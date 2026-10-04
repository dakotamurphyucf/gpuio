# Implementation status

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


## Selection child policy (2026-10-01)

The public selection preview now combines rich command-button content, independent
Bold loading and Italic disabled state, group-wide availability, connected/separated
geometry and stable item keys. No new protocol operation or native widget is used.
The model rejects delayed requests against current child/group policy; bulk changes
preserve unavailable selections, and the master state describes the whole set.

Full `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
examples/gallery/main.exe` passes after the final changes. The new expect case
checks queued child requests, partial and repeated bulk selection, group disable,
policy recovery, then complete select/clear. Existing selection cases remain green.
`scripts/gallery_selection.py` adds busy/disabled AX properties, owner retention,
busy focus, disabled Tab skipping, partial bulk updates and recovery. Python
compilation passes; the desktop assertions are authored and **unrun**. This does
not establish physical macOS behavior, spinner rendering or full family acceptance.

The final selection example and its model were copied into the fresh staged
hover consumer (public library code was unchanged), then rebuilt with the staged
`OCAMLPATH` and repository toolchain. That independent build passed with
`SELECTION_POLICY_INSTALLED_CONSUMER_BUILD_PASS run=False`. No runtime or desktop
acceptance is implied.


## Live command tooltip (2026-10-01)

The public Controls card composes `command_scope`, the Bonsai binding observer,
a stable command button, managed Tooltip and observed keyboard presentation.
The observer remains outside tooltip content so it can sample while the tooltip
is closed. Here-context registry declarations supply the hint; locally configured
shortcut values are not used to render it. The example distinguishes pending,
absent, disabled and unavailable observations, and changes labels independently
of registration. It only increments a local request counter.

Executed locally:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe` passed.
- Python compilation of `scripts/gallery_command_tooltip.py` and its dispatcher
  passed. The new walkthrough is included in `--section buttons`/`all`, but unrun.
- Copied the final `pages.ml` and `command_tooltip_preview.ml` into the fresh staged
  hover consumer, whose public libraries are unchanged by this example. Its
  independent Dune build with the staged `OCAMLPATH` passed:
  `COMMAND_TOOLTIP_INSTALLED_CONSUMER_BUILD_PASS run=False`.

No native library/protocol changes or new GUI launch. Actual tooltip updates,
keyboard delivery, accessibility and installed-consumer runtime remain required.
Existing native binding tests are evidence for the underlying observer, not
physical acceptance of this new composition.


## Choice picker data domain (2026-10-01)

The additive `Choice_picker` module implements validated Core values only. It
does not yet have a native View, controller or wire encoding. Four expect tests
cover separate group/item identity, cross-group duplicates, complete-catalog
selection membership, ordered toggles, mode/disabled/removed-item guards, explicit
clear policy, preserved selection under regrouping, maximum group/item/selection
counts, aggregate group metadata bytes and malformed/oversized text.

`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/view_api/runtest @fmt`
passed after correcting an initially unavailable Core helper to `List.fold_result`.
The subsequent full `dune build -j2 @runtest @fmt examples/gallery/main.exe`
through the same isolated wrapper passed. No expectations were promoted. No
Rust/GUI/Linux/current installed-consumer run is claimed for this domain addition.
The [design contract](../design/choice-picker.md#validated-core-domain-checkpoint)
records the exact limits and why pure reduction is separate from the future
transport-generation, modal, composition and callback lifetime guards.


## Choice picker configuration codec (2026-10-01)

Standalone OCaml and Rust configuration types now preserve grouped/flat catalogs,
ordered multiple/single selection, disabled/search/clear/open preferences and
separate trigger/search placeholders. No native tree operation or View is added.
`Choice_picker.Expert.of_wire` revalidates before producing typed data.

Executed locally:

- Focused OCaml view API expect tests and formatting pass, including an added
  independent grouped Unicode/multiple fixture, flat empty/single fixture and
  rejected invalid conversions.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol`
  passes in full, including three picker tests: exact bytes, all truncations,
  trailing data, malformed UTF-8/Boolean/enum tags, duplicate group/item/selected
  IDs, absent selected values and all search/open-state variants.
- Allocation-bound cases pass for group/item declarations exceeding the limits,
  cumulative group metadata text, nested groups exceeding the total item budget,
  selected-ID count/text budgets and oversized entire payloads. A valid 4,096-item
  catalog selecting all IDs roundtrips.
- Protocol all-target strict Clippy and workspace Rust formatting pass.

These tests validate serialization and standalone admission rules. They do not
verify native tree quota charging, event routing, focus, rendering, IME or an
installed picker consumer; those integrations remain required.

The subsequent full isolated OCaml `dune build -j2 @runtest @fmt
examples/gallery/main.exe` also passed, including the rebuilt native backend.
This establishes build compatibility for the standalone codec additions; it is
not a picker rendering or installed-consumer runtime test.

## Choice picker events and transition state (2026-10-01)

Core event values and checked conversion now distinguish selection/open intent
from actual visibility snapshots/edges. The standalone bounded Rust decoder
rejects malformed IDs, tags, Booleans, truncation, trailing bytes and impossible
reason/direction combinations. Sixteen independent hex fixtures include Unicode
selection and each dismissal reason. No bridge event envelope is introduced.

Five pure native state tests pass for single Select/close ordering, repeatable
multiple Toggle requests without optimistic selection, controlled request retries,
accepted configuration edges, Controlled-to-Managed handoff, mount-only managed
initial state, disabled/culled closure/recovery, removed/disabled choices, composing
activation/clear, explicit clear policy and atomic invalid-configuration rejection.

Executed locally through `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --locked -j2 -p gpuio-protocol --test choice_picker`: four passed,
  including the existing configuration tests and new independent event fixture.
- `cargo test --locked -j2 -p gpuio-native --lib choice_picker_state`: five passed.
  These tests use no GPUI window or OS input.
- `cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets -- -D warnings`
  and `cargo fmt --all -- --check`: passed.

The first OCaml expectation run found the sexp printer escapes the Unicode ID;
the expected output was corrected explicitly after inspecting the diff. No
expectations were promoted. The subsequent full `dune build -j2 @runtest @fmt
examples/gallery/main.exe` passed through the same isolated wrapper, including
the rebuilt native backend and the six picker expect cases.
Native rendered ownership, query leases, bridge
delivery/overflow/lifetime fencing, actual macOS behavior and current Linux or
installed-consumer acceptance remain unverified for this extension.

## Picker query provenance (2026-10-01)

Selection_request now carries the stable-ID intent and an optional exact query
snapshot; Query_changed can carry active composition. Native state binds the
accepted query-child NodeId and validates its presence against search mode. Core
conversion combines the routed window with the expected child identity and
validates the snapshot before exposing it. Selection/clear reject composition.
The standalone event schema is still unpublished and has no bridge envelope.

The existing editor wire definitions moved unchanged to `Editor_wire`, with
transparent `Wire.Editor` aliases, to avoid a dependency cycle. The query payload
uses those shared fields. It admits at most 262,144 text bytes and validates
single-line UTF-8, NUL exclusion, nonnegative revision, scalar-boundary selections
and ordered composition. The whole standalone event limit is 262,656 bytes.

Executed through the isolated `GPUIO_JOBS=2 ./scripts/gpuio exec` wrapper:

- `cargo test --locked -j2 -p gpuio-protocol --test choice_picker`: five passed,
  including independent selection/query/composition/clear bytes, all truncations,
  malformed node generation/text/offset/Boolean cases and maximum-size query.
- `dune build -j2 @test/view_api/runtest @test/protocol/runtest @fmt`: passed,
  including seven picker expect cases and existing editor/protocol fixtures.
- `cargo test --locked -j2 -p gpuio-native --lib choice_picker_state`: six passed.
  The new regression captures a query before later edits, rejects composing and
  absent queries, rejects the old editor after remount despite equal revisions,
  and atomically rejects mismatched search configuration/child identity.
- `cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets -- -D warnings`
  and `cargo fmt --all -- --check`: passed.

These are data, serialization and pure native-state results. No OS window was
opened. Connecting the retained child editor to popup rendering, ordered delivery,
hidden-input focus gates and tree/handler lifetime checks remains required;
no current Linux, physical macOS or installed-picker acceptance is claimed.

The subsequent full isolated `dune build -j2 @runtest @fmt
examples/gallery/main.exe` also passed, including the rebuilt native backend and
all consumers reached by those targets. No expectation output was promoted.

## Picker grouped rows and variable-height layout (2026-10-01)

`choice_picker_rows` projects the full validated catalog into bounded group/item
rows with separate identity namespaces, native lowercase filtering and cached
selection flags. The cursor preserves IDs through reorder/filter, skips disabled
items/headers, respects selection order, and supports wrapping plus first/last.
Hidden selections remain committed; zero items with retained headers still count
as empty content. Tests cover flat/grouped data, duplicate labels with distinct IDs,
Unicode/no-normalization behavior, all search modes and maximum 256 groups plus
4,096 items.

`choice_picker_list` uses retained GPUI ListState with measured variable heights.
Structural edits splice the changed span and restore the stable-key scroll anchor;
content-only changes remeasure without resetting scroll. Explicit rich-content
invalidation and explicit cursor reveal are separate from ordinary rendering.

Executed locally through `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --locked -j2 -p gpuio-native --features native-image-tests --lib choice_picker`:
  14 passed. This includes the prior six state tests, five row-projection tests,
  two list-state tests and one real GPUI layout test using TestPlatform.
- The layout fixture retains one native render-view entity, verifies measured
  18/28/52-pixel rows, describes fewer than 32 rows per draw with 4,096 choices,
  and retains the visible key/offset after a preceding row shrinks and an earlier
  item is removed. It creates no macOS window and performs no GPU/AX acceptance.
- `cargo clippy --locked -j2 -p gpuio-native --features native-image-tests --all-targets -- -D warnings`:
  passed.

The first compile used equality on GPUI ListOffset, which lacks PartialEq;
the test now compares its fields. The first layout execution exposed a fixture
setup error: a standalone list lacked a rendering view owner. Retaining one GPUI
view corrected the setup; all 14 tests then passed. No GPUI/vendor patch was made.
This does not implement the popup widget, rich View slots, native admission,
query-editor routing or accessibility; public gallery/consumer and physical
platform acceptance remain required.

The subsequent isolated `dune build -j2 examples/gallery/main.exe @fmt` and
workspace Rust formatting check also passed. This native-only addition did not
rerun the full OCaml suite; its previous full passing checkpoint remains the
query-provenance change above. No new desktop or installed-consumer runtime
acceptance is implied by rebuilding the gallery.

## Picker render description and standalone child admission (2026-10-01)

Core Query/Option_content/Appearance/Description now express mount-only search
seed, native/custom selected indicator, variable-height popup geometry and typed
group/item content overrides. Description checks current membership, uniqueness
and exact query/search-mode correspondence. The generic content type deliberately
does not depend on View; actual passive View checks remain part of the pending
View constructor.

OCaml/Rust presentation encoders match an independent fixture containing different
geometry values, four distinct paint-part styles and seven slot roles. Rust caps
the payload, slot counts/text and style declarations before further admission.
`choice_picker_admission` validates styles and supplied wrapped child trees,
including passive visual slots, a matching single-line query Input with handler,
interactive footer, parent/identity consistency and bounded forest traversal.
It is not yet invoked by a picker operation or atomic tree plan.

Executed locally through `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --locked -j2 -p gpuio-protocol --test choice_picker`: seven passed;
  presentation fixture, truncation/trailing data, geometry, repeated/missing slot
  roles/IDs and allocation bounds join existing config/event/query coverage.
- `dune build -j2 @test/view_api/runtest @fmt`: passed, with ten picker expect
  cases covering descriptions, query placement, custom indicators, part styles,
  exact bytes and invalid geometry/structural styling. No expectations promoted.
- `cargo test --locked -j2 -p gpuio-native --test choice_picker --test appearance --test choices`:
  three new picker checks plus three existing choice/appearance checks passed.
  Cases include late descendant callbacks/input styles, mismatched query settings,
  parent aliases, excessive footer descendants and combined style budgets.
- `cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets -- -D warnings`
  and workspace Rust formatting: passed.

No OS window opened. The child validator was tested against retained tree nodes
and modified copies; these results do not establish atomic picker rollback,
resource charging, native popup/focus/IME behavior or a public widget. Those
integrations and physical macOS/required Linux/consumer acceptance remain open.

The subsequent full isolated `dune build -j2 @runtest @fmt
examples/gallery/main.exe` also passed with the rebuilt native backend.


## Picker atomic tree admission — 2026-10-01

Kind53 and Op72 now pair OCaml/Rust presentation submission with the real native
Tree plan. An independently specified envelope verifies tag ordering and the
existing presentation fixture in both languages. No public picker View or
rendered widget is claimed yet.

`cargo test --locked -j2 -p gpuio-protocol --test choice_picker` passed eight tests.
`cargo test --locked -j2 -p gpuio-native --test choice_picker` passed five tests.
Both used `GPUIO_JOBS=2 ./scripts/gpuio exec` on local macOS arm64 without OS
windows. New native cases apply actual transactions: descendant callbacks,
interaction styles, wrapper styles, query autofocus, missing root handler,
nonempty root text, wrong-kind configuration and disconnected children reject
without changing accepted nodes/revision/bytes. Interactive footer callbacks and
passive text updates succeed. Configuration/query policy updates are atomic;
a retained-payload budget rejection rolls back, a larger budget admits the
update, and complete removal releases the charge.

Payload accounting covers retained presentation/catalog/selection/style/slot
storage. It does not yet reserve mounted projection/list/editor caches, which
must be integrated with the pending native owner. No new physical macOS, Linux,
installed-consumer or hosted CI acceptance follows from these checks.

The full isolated `dune build -j2 @runtest @fmt examples/gallery/main.exe`
subsequently passed, including the OCaml operation fixture and rebuilt backend.
Strict protocol/native all-target Clippy and workspace Rust formatting passed.
The native library plus picker/choices/appearance/session regression run passed
480 tests with two existing ignored tests. This run used default features and
opened no OS windows; it does not supersede physical or feature-gated acceptance.


## Picker event bridge and session leases — 2026-10-01

Event69 now carries picker events with window/node/handler/tree revision. Rust
encoders and the OCaml event reader share independent full-envelope fixtures;
nested query composition is permitted for observations and rejected for selection.
Eio recognizes the envelope; actual public View callback installation remains
pending, so this is not an end-to-end application interaction claim.

Using the isolated wrapper on local macOS arm64:

- `cargo test --locked -j2 -p gpuio-protocol --test choice_picker`: nine passed.
- `cargo test --locked -j2 -p gpuio-native --lib --test choice_picker`: 473 passed,
  two existing ignored tests. New session coverage includes current-mode and
  enabled-item checks, missing/composing query rejection, ordinary model/footer
  commits preserving repeated intents, remounting the query at the same revision,
  disabling with final closure/query observations, handler rebinding, overload
  and window closure.
- The mailbox regression retains repeated toggles and subsequent closure in order,
  charges both ID and query text, reports byte-limit overflow without discarding
  admitted events, emits bounded encoded batches and releases all input charges.

No OS windows were opened. Modal/focus gating, exact native snapshot capture and
transport-failure handling by the mounted owner, Core callback dispatch and Eio
controller observation still need integration. Physical desktop and current Linux/
consumer/CI acceptance remain open.

The subsequent full isolated `dune build -j2 @runtest @fmt
examples/gallery/main.exe` passed, including eleven picker expect cases. The
new envelope cases cover exact bytes, truncation/trailing input, negative tree
revision, invalid direction, composing selection and invalid UTF-8 selection
boundaries. Strict native/protocol all-target Clippy and workspace Rust formatting
also passed. No expectations were promoted.


## Picker accepted-tree host ownership — 2026-10-01

The host now installs/updates/retires picker owners with accepted tree identities,
retains query editors, and synchronizes closed-slot visibility in the shared focus
manager. Parent-depth ordering covers nested pickers in interactive footers.
Mount/rebind and configuration visibility observations reach Event69 through the
session/mailbox checks; failure uses the explicit overload path. Root admission
now reserves cloned configuration plus owner/focus metadata in addition to the
presentation payload. Projection/list reservations await rendering integration.

`GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native
--features native-image-tests --lib` passed 517 tests with two existing ignored
tests. The three new tests use TestPlatform and the production View/editor path:
closed Focus is rejected; opening preserves the editor focus handle/draft;
closing blurs before layout; managed initial preference is not replayed; nested
picker gates settle parent first despite reverse allocation; another visibility
set remains independent; 200 idle syncs do not repeat zero-slot mount observations;
rebinding emits one new snapshot; removal clears owners/editors and tree payload.

An initial test compile had an ambiguous Event glob import, corrected with an
explicit alias. The first full run then exposed a fixture mistake: a fabricated
container-query visibility entry was correctly removed by that feature's ordinary
synchronization. The independence check now exercises picker gate replacement
without asking the container-query synchronizer to preserve nonexistent state.
The final full run passed. No production invariant or expected behavior was
relaxed. A separate production adjustment blurs already-focused hidden content
on closure; the editor focus test verifies it before any layout.

This is native host/lifecycle evidence without OS windows. Popup drawing, exact
painted visibility/culling, query gesture/IME handling, trigger focus restoration,
Core callback/public View integration, installed consumers and real platform
qualification remain open.

The subsequent isolated `cargo test --locked -j2 -p gpuio-native --test
choice_picker` passed six admission/session cases with the new owner reservation.
Strict native/protocol all-target Clippy with `gpuio-native/native-image-tests`
enabled passed. Full `dune build -j2 @runtest @fmt examples/gallery/main.exe`
and workspace Rust formatting passed. These remain local, uncommitted checks;
no current hosted or Linux result is implied.


## Picker popup and keyboard integration — 2026-10-01

The production host renders the accepted picker description: retained native
query, grouped variable-height virtual rows, rich passive content, custom trigger,
empty content and interactive footer. A rendered TestPlatform fixture uses 4,096
options, checks fewer than 32 visited rows, a measured 52-pixel rich row, a total
160-pixel popup limit, current query filtering and exact query snapshots on rapid
selection requests. Accepted selection remains application-owned.

The fixture reproduced footer Enter incorrectly selecting the active option.
Restricting option keys to trigger/query focus preserves the footer's own Press
callback. It also exposed cached/deferred paint order changing Tab order. The
focus manager now orders popup descendants by accepted child order and local Tab
indices at their trigger's position. Tests verify query-to-footer-to-outside Tab,
closure without stealing destination focus, Escape restoring the trigger while
preserving draft text, and nested pickers keeping their own keyboard requests.
The test activates only TestPlatform's fake window to exercise GPUI focus-out
listeners; it does not open or activate a macOS desktop window.

Closed-cache testing updates a previously rendered 4,096-item picker without a
redraw, verifies the old configuration Arc is released and logical accounting
shrinks, then verifies owner/editor teardown and zero retained tree accounting.
Render cache reservations are conservative logical bounds, not RSS measurements.

Commands ran locally through `GPUIO_JOBS=2 ./scripts/gpuio exec`, with one heavy
command at a time:

- `cargo test --locked -j2 -p gpuio-native --features native-image-tests --lib`:
  519 passed, two existing ignored tests; includes five picker host tests.
- `cargo test --locked -j2 -p gpuio-native --test choice_picker`: six passed,
  including budget rollback and release under the renderer reservation.
- `cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets
  --features native-image-tests -- -D warnings`: passed after collapsing one
  nested focus-restoration guard, with no behavior change.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passed, including
  rebuilt native backends, OCaml tests, formatting and gallery linking.

Public picker View/reconciler/Eio/Bonsai and gallery bindings remain pending.
Measured clipping/culling, complete accessibility and clear-keyboard policy,
composition/query observation, geometry invalidation and broader resource cases
also remain. These results establish neither physical macOS input/IME/AX nor
current Linux/installed-picker/hosted-CI acceptance. The separately tracked
black-window startup observation remains unresolved. The full milestone and
OCH-41 are incomplete.

## Public picker API, query stream and gallery — 2026-10-01

`Gpuio.View.choice_picker` and the Bonsai alias now mount checked descriptions.
They validate passive slots and aggregate node/depth budgets, create genuinely
unstyled structural wrappers, and mount a single-line query under its own lease.
Internal slot identities use namespace/ID pairs, preserving full 256-byte group
and item IDs without hashing or colliding with user keys. Eio's default root key
uses the controller's stable Bonsai placement identity.

The reconciler binds the actual mounted query NodeId after reconciling children.
Two new expect tests (thirteen picker cases total) verify prepared callbacks remain
isolated until acceptance, queued toggles survive ordinary model updates, disabled
changes retire handlers, query remounts reject old snapshots, duplicate controller
placements fail, passive input is rejected, and native drafts are not reset by a
new seed. Current mode/item/clearability policy is checked again at dispatch.

A frozen public-View transaction is emitted by that OCaml test and independently
decoded/applied to Rust Tree. Native admission verifies its real query policy,
slot shape, rich option and interactive footer. This is stronger than separately
building hand-written transactions, but does not establish desktop behavior.

Query observations travel through the root Event69 route with selection/open/
visibility. The retained editor emits no duplicate ordinary Editor_event for its
query; internal query bindings discard such callback events. Owner mount/rebind
visibility snapshots precede query snapshots. Native tests verify three query
changes queued before draining remain three ordered observations. The existing
bounded FIFO/overflow policy applies. First Escape cancels marked composition
without closing the popup; old observer Tab callbacks cannot move focus.

`Gpuio_eio.Choice_picker` reuses the native editor controller and observes query
snapshots before application effects. Commands and conditional replacement retain
editor generation/revision checks. The Pickers gallery adds a public-API grouped
capability chooser with multiple selection, rich rows, search, a disabled item and
an interactive clear footer. Its reducer applies each intent to current state.
The new example has not yet undergone visual/desktop input qualification.

Local commands, using the isolated repository environment:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native
  --features native-image-tests --lib --test choice_picker`: 519 library tests
  passed, two existing skips; seven admission/integration tests passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native
  -p gpuio-protocol --all-targets --features native-image-tests -- -D warnings`:
  passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
  examples/gallery/main.exe`: passed after the structural-identity change.
- `cargo fmt --all --check` through the same wrapper and `git diff --check`:
  passed. Catalog structural audit: 146 root-module entries, 43 owned families;
  these counts do not establish functional acceptance.

Remaining work includes complete native AX/clear-keyboard behavior, measured
clipping/culling, ancestor geometry invalidation, controlled focus cases, broader
gallery variants, actual OS composition/focus/accessibility and resource workloads.
No desktop window was opened for these checks. Linux, clean-machine/packaged
runtime and hosted-CI/review acceptance are not established by these results.
OCH-41 and milestone 07 remain incomplete.

The fresh staged installation also passed `GPUIO_JOBS=2 python3
scripts/test_extension_consumer.py --example gallery --workspace
scratch/agents/root-20260929-m7-resumed/picker-installed-gallery` (exit 0,
`INDEPENDENT_EXTENSION_CONSUMER_PASS`, `run=False`). This verifies that the
independent gallery builds against the installed public libraries and rebuilt
backend; it does not establish installed-consumer runtime acceptance.


### Picker trigger clipping and native scrolling — 2026-10-01

The production host now samples trigger paint after deferred popup content and
combines measured visibility with accepted-tree eligibility. Fully clipped or
unpainted triggers close retained popups with one Unavailable observation. Managed
initial-open is not replayed on recovery; controlled opening is restored when the
trigger becomes visible. Unchanged paint emits no edge and requests no redraw.

The new TestPlatform fixture exercises full/partial clipping, silent repeated
paint, both opening policies, and native scroll offsets at an unchanged tree
revision. It also checks that the fixture has real scroll extent. This uses
production layout/paint with no OS window, not physical macOS GUI/AX acceptance.

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native
  --features native-image-tests --lib --test choice_picker`: 520 library tests
  passed, two existing skips, seven admission/integration tests passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native
  -p gpuio-protocol --all-targets --features native-image-tests -- -D warnings`:
  passed.
- Repository-wrapper `cargo fmt --all` and `git diff --check`: passed.

The prior independent gallery build preceded this native-only change. Broader
nested culling/OS occlusion, complete picker AX/clear-keyboard behavior, geometry
invalidation, physical native testing and release acceptance remain open.


### Picker keyboard Clear and semantic metadata — 2026-10-01

Clear now has a retained native focus handle, keyboard focus indicator and
independent Focus/Click accessibility actions. Enter/Space emits a clear intent,
with the current query snapshot when present, without changing committed selection
or popup state. Removing clearability restores the trigger when eligible;
disabling the owner removes Clear focus. Host compound-part order is consistent
under clipping as well as normal native traversal.

The searchable regression exposed GPUI Base's Shift-Tab/OutdentInline action
consuming the gesture before the raw key listener. The picker now captures
IndentInline/OutdentInline only when its query is focused, preserving footer
editor behavior and existing composition/generation guards. The regression checks
query -> Clear -> query, unchanged popup, clear snapshot and footer activation.
The standalone fixture checks Enter/Space, pending-model semantics, removal,
disabled focus and closed/open traversal with a genuinely clipped sibling.

The native semantic wrapper retains real element identities/layout/actions and
provides listbox multiselectability, selected/disabled option state, and disabled
Clear semantics. Three production Element metadata tests verify roles, labels,
identity, selected/disabled state and supported actions; they do not run an OS AX
client. Disabled triggers no longer install a Click accessibility callback.

Final native command: `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2
-p gpuio-native --features native-image-tests --lib --test choice_picker` passed:
524 library tests, two existing skips, seven admission/integration tests. No
desktop window opened. Full group relationships, query active-descendant focus,
virtualized positions and external AX/VoiceOver remain open. This is not complete
picker, catalog or milestone acceptance.

Final strict native/protocol all-target Clippy with `native-image-tests` and
`-D warnings` passed. Repository-wrapper `cargo fmt --all` and `git diff --check`
passed. `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
examples/gallery/main.exe` also passed after rebuilding both native backends.
The gallery was built, not launched; the independent installed-consumer build
above predates these Clear/semantic changes.


### Virtual row identities, membership and text reservation — 2026-10-01

Rows use the full accepted ID with separate group/item tags in GPUI's named integer
identity. Filtering/reordering no longer aliases different options by row index.
The projection provides logical option positions/counts, including disabled items,
within the filtered group or flat collection. Group descriptions remain available
on mounted options even if their visual header is not mounted. Tests cover
filtering, reorder, flat/empty results, namespace separation with 256-byte IDs,
positions/counts and native Element metadata. This does not create actual AX group
parentage or repair the query/list sibling active-descendant relationship.

Native renderer reservation includes three ID copies and four display/semantic
label copies per row, including repeated group descriptions. An admission fixture
verifies a valid 100-option catalog cannot accept a 1,024-byte group-label update
under a budget sufficient for only the catalog text; failure preserves revision
and retained bytes. A sufficient budget accepts it, and removal releases the charge.
These are logical reservations, not process-RSS measurements.

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native
  --features native-image-tests --lib --test choice_picker`: final production
  changes passed 526 library tests, two existing skips and seven then-existing
  integration tests.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native
  --test choice_picker`: all eight integration tests passed after adding the
  repeated-description budget fixture.
- The gallery build/format checkpoint passed before the final repeated-label
  accounting adjustment; final rebuild/lint results are recorded below.

All checks above are local build/TestPlatform/admission evidence, with no new OS
window, external accessibility client, Linux qualification or release acceptance.

Final strict native/protocol all-target Clippy (`native-image-tests`, `-D warnings`)
passed after the text-reservation change and new admission fixture. The final
`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt examples/gallery/main.exe`
also passed with the rebuilt native backend; `git diff --check` passed. The last
full OCaml test run passed in the preceding Clear checkpoint, with no subsequent
OCaml implementation changes. No GUI run or current installed-consumer runtime
acceptance is claimed.


### Explicit query ownership for active-option accessibility — 2026-10-01

The pinned GPUI adaptation now exposes `aria_active_descendant_for(&FocusHandle)`.
It weakly references a related input, requires that exact live handle to own the
window's keyboard focus and match the accessibility tree's real focused node,
and then reports the option as accessibility-focused. Missing/self/unfocused
owners are ignored. It does not move keyboard focus or fabricate input ancestry;
ordinary ancestor behavior and duplicate-claim guards remain intact.

GPUIO supplies this owner only for its focused query outside marked composition.
A production helper reads the editor's composition flag without copying its draft.
The existing native fixture now verifies eligibility before composition, rejection
while marked, recovery after first Escape, and rejection while Clear is focused.

An isolated copy of the current vendor source passed all 17
`window::a11y::tests` with `--locked --offline -j2` and
`test-support,gpui_platform/runtime_shaders`. New tests cover the focused input's
value and unchanged ancestry, unrelated/missing/self owners, frame retirement,
duplicate claims, and real element prepaint on TestPlatform. The rendered fixture
checks both ancestor-only and explicit sibling ownership plus focus on another
control, while asserting unchanged real keyboard focus. No OS window was created.

The integrated `cargo test --locked -j2 -p gpuio-native --features
native-image-tests --lib --test choice_picker` passed 526 library tests, two
existing skips and eight integration tests. These results include the composition
helper checks; a subsequent vendor edit only removed an unused test import, after
which the 17 isolated GPUI tests passed again without that warning.

The final cumulative patch hash is
`b59b2178351ff990964ecf1746b5b739f496ca39aced0c31f8bf747490d79c6d`.
The normal reconstruction script verified archive/patch hashes and reproduced
`vendor/gpui` exactly. Explicit rustfmt checks of changed vendor/native files and
`git diff --check` passed. Dependency versions and source pins are unchanged.
External AX notifications, VoiceOver, physical IME and complete grouped semantics
remain unverified; these are local builder/TestPlatform results.

Final strict native/protocol all-target Clippy (`native-image-tests`, `-D warnings`)
passed. A fresh staged installation also passed
`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
--workspace scratch/agents/root-20260929-m7-resumed/picker-a11y-installed-gallery`:
exit 0, `INDEPENDENT_EXTENSION_CONSUMER_PASS`, `run=False`. This builds the public
gallery and its independent backend against the final GPUI patch and current
libraries, covering the Clear/membership/adaptation changes. It is build evidence,
not installed-consumer runtime or clean-machine distribution acceptance.

### Mounted picker accessibility groups and single slot ownership — 2026-10-01

The picker now emits named Group parents containing its actual mounted option
nodes. Stable full catalog IDs survive scrolling a header away, label changes and
reordering. Hidden decorative headers avoid duplicate announcements; options keep
logical position/count and existing actions/selection. Flat collections keep
options as direct list children. Capture shares the immutable projection by index,
so group text is no longer copied or reserved once per option.

GPUI's test adapter now retains normal accessibility activation callbacks and
submitted tree updates. The real native View fixture checks header culling,
repeated frames, bounded mounted options, multiple/header-only groups, flat
conversion, rename/reorder identity, disabled actions, adapter reconnect and
retirement. Each observed tree is checked for missing children, duplicate parents
and orphaned nodes. These are pre-platform tree checks, not VoiceOver evidence.

Enabling the adapter in the existing 4,096-option searchable fixture reproduced
`set_focus called more than once in a single frame`: the query input appeared
both in the popup and the host's generic child traversal. ChoicePicker structural
slots are now excluded from that traversal. Assertions verify one query input,
one footer, option accessibility focus with the query still owning keyboard focus,
and input accessibility focus during marked composition. Physical IME remains open.

`GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 -p gpuio-native
--features native-image-tests --lib --test choice_picker` passes 527 library tests
(two existing ignores) and eight admission tests. The updated resource test rejects
a label update that reserves catalog bytes alone, preserves the previous revision
and payload on failure, admits a bounded shared-label reservation and releases it
on removal. Local log: `picker-group-native-final.log` in the ticket notepad directory.

Two isolated GPUI builder tests also pass, covering preservation of rich subtrees,
focus/actions/order, stable group identity and retirement, plus atomic rejection
of invalid ranges and ID collisions. The final isolated 19-test GPUI accessibility
suite passes with the new TestWindow adapter. Strict native/protocol all-target
Clippy with `native-image-tests` and `-D warnings`, workspace Rust formatting,
explicit vendor formatting and `git diff --check` also pass.
The cumulative patch hash is
`53b0aac8cfa1823be60b7aab8e9c10b0792e0ea095bb8734f7054124ce037089`.
The hash-verified pinned archive reconstructs exactly into the current vendor tree.
No OS window was opened. External AX/VoiceOver, physical input, popup geometry,
consumer runtime, remaining catalog families and macOS release acceptance remain
incomplete. This does not resolve the black-window startup issue.

The fresh independent gallery consumer also builds successfully:
`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
--workspace scratch/agents/root-20260929-m7-resumed/picker-group-installed-gallery`
reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
It stages the current public libraries and independently builds/links the backend
with the current fork. This is build evidence, not installed runtime or clean-machine
GUI acceptance. Log: `picker-group-consumer.log` in the same local notes directory.

### Bonsai picker controller sequencing and retirement — 2026-10-01

Four private inline expect tests exercise the production controller component
through Window_driver, its reconciler and Bonsai scheduler. Command completion
is controlled at a typed private capability boundary; the public Eio adapter still
binds `App.Window.Expert.editor_command` and has unchanged public signatures.

The tests establish that application effects using `Bonsai.peek` observe the new
query/selection snapshot, older native revisions cannot roll it back, and late
command replies cannot replace newer typing or a replacement editor's observation.
A delayed command effect retains its original editor lease. Stale routed query
events are rejected by the real reconciler. Conditional replacement sends the
selection request's exact revision and leaves newer typing untouched on failure.

The search-disabled case reproduced an unwanted outbound command from
`replace_if_unchanged`. It now returns `Not_mounted` without sending; after a new
query generation is observed, the old selection returns `Stale_editor`.

Validation: `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt
examples/gallery/main.exe` passed, including all four new tests. The dedicated
log is `scratch/agents/root-20260929-m7-resumed/picker-controller-dune-final.log`.
Expect output was reviewed explicitly; no blanket promotion was used. Existing
native linker duplicate-library and Rust `block` future-compatibility warnings
remain. These tests do not run Eio I/O, validate native command responses, close
real windows or establish physical keyboard/IME behavior. No OS window opened.

A fresh staged public-library installation and independent gallery/backend build
also pass: `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
--workspace scratch/agents/root-20260929-m7-resumed/picker-controller-installed-gallery`
reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
Log: `picker-controller-consumer.log` in the same local notepad directory.
This checks packaging of the new private modules; it is not consumer runtime or
clean-machine GUI acceptance.

### Inherited picker text layout — 2026-10-01

A rendered native View regression reproduced stale offscreen row heights after
an ancestor-only line-height update. Visible rows changed from 28 to 48 logical
pixels, while previously visited row 50 retained 28. The old comparison read
`window.text_style()` during View construction, before ancestor styles resolved.

The list now compares resolved inherited metrics during element layout, including
rem size/display scale and excluding paint-only fields. It invalidates heights
before ListState's layout borrow, preserving its item/pixel-offset anchor. The
regression passes growing/shrinking inherited line height, returning to old rows,
popup overrides masking ancestor changes and idle-frame cache retention.

`GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 -p gpuio-native
--features native-image-tests --lib --test choice_picker` passes 528 library tests,
two existing ignores and eight admission tests. Strict native/protocol all-target
Clippy with `native-image-tests` and `-D warnings` passes. Local logs are
`picker-layout-repro.log`, `picker-layout-fixed.log`, `picker-layout-native-final.log`
and `picker-layout-clippy.log` under the current OCH-41 notepad directory.
No GPUI fork, public API or dependency change. No OS window opened; actual native
scrolling/display-scale/resize qualification and full catalog acceptance remain open.

Workspace Rust formatting and `git diff --check` pass. The fresh staged gallery
consumer also passes with `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery
run=False`: `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
--workspace scratch/agents/root-20260929-m7-resumed/picker-layout-installed-gallery`.
The corresponding `picker-layout-consumer.log` records independent public-library
and backend build/link coverage. It was not launched; this is not runtime or
clean-machine graphical acceptance.

### Public picker gallery scenarios — 2026-10-01

The existing grouped multiple-selection card is joined by controlled single
selection, a 4,096-option searchable directory and an empty catalog with a
create-first-workspace footer/reset flow. All use public `Gpuio_eio.Choice_picker`
and View APIs with the shared palette and scale. The large catalog is data only;
row Views remain native and virtualized.

The controlled destination model keeps requested opening and native visibility
separate, disables changes visibly and rejects queued intents using current
permission/configuration. Two new expect tests cover clipping observations without
losing open intent, permission/disabled/wrong-mode/missing-item requests, reset,
and validated public descriptions for maximum-size, empty and populated catalogs.

`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt
examples/gallery/main.exe` passed; the final label-only adjustment also passed
`@test/gallery/runtest @fmt` and the gallery build. Logs: `picker-gallery-cases-final.log`
and `picker-gallery-final.log` in the current OCH-41 notes directory.

The new `scripts/gallery_choice_picker.py` walkthrough is integrated as
`--section choice-pickers`. It checks selection, policy, large-catalog filtering,
retained draft and empty/create/reset behavior. The older date/color walkthrough
now reveals its trigger after the added cards move it below the viewport. Both
Python files compile. These desktop checks are **unrun**; their AXValue search
setter would not establish physical typing or IME coverage. No new screenshot,
visual polish acceptance, OS input or clean-machine runtime claim is made.

The final fresh installed consumer also passes (`picker-cases-consumer.log`,
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`). This stages the
current public libraries and builds the gallery/backend in an independent Dune
workspace; it does not launch the application or establish clean-machine runtime
acceptance. `python3 scripts/audit_component_catalog.py` and `git diff --check`
pass after the catalog/evidence changes.


### Controlled picker focus — 2026-10-01

An actual native View regression reproduced a controlled-mode focus gap: accepting
an open request in a later transaction showed the popup but left focus on its
trigger. Managed-mode synchronous gesture handling already focused search.

The owner now retains at most one guarded handoff until the requested visibility
transition. Search receives accepted opens, and the trigger receives accepted
Escape closes. A weak source handle plus blur cancellation prevents late replies
from reclaiming focus after the user moves away, including away and back. Callback
replacement, query replacement, disabled/ineligible state and leaving controlled
mode cancel pending handoffs; initial/programmatic visibility does not create one.
There is no new public API, protocol field or GPUI fork change.

Executed locally on macOS using TestPlatform (no desktop windows):

- Focused regression failed before the repair at “accepted controlled open focuses
  query” (`picker-controlled-focus-repro.log`), then passed.
- Final `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --locked
  --offline -j2 --features native-image-tests --lib --test choice_picker`:
  **529 library tests passed, two existing skips; eight admission tests passed**
  (`picker-controlled-focus-native-final.log`). The regression covers accepted
  open/Escape close, focus away/back, stale close after focus moves, unsolicited
  visibility changes, unchanged updates, observer rebinding, disabled transitions
  and leaving controlled mode. Query replacement is guarded by implementation;
  this new regression does not separately exercise that fence.
- Strict all-target native/protocol Clippy with `native-image-tests` and
  `-D warnings` passes (`picker-controlled-focus-clippy.log`).

Logs are in the implementing agent's ignored scratch directory. Physical keyboard,
IME, external accessibility, desktop rendering and the black startup issue remain
open; these tests do not establish that acceptance.


### Picker resize and wrapped labels — 2026-10-01

A native View regression found long labels retaining a one-line 28-pixel height
when the popup narrowed: the content flex child kept its minimum content width.
Shrinkable option/header content and a fixed-width native checkmark repair that
layout. The regression then passed width/height restoration, remeasured wrapping,
item 50 plus five-pixel scroll retention, unchanged tree revision, simulated scale
changes, bounded query/footer geometry and retained query identity. Three settled
draws schedule zero next-frame callbacks. The small tested viewport is 240×140;
this does not establish usability for arbitrary oversized custom content.

Executed locally with TestPlatform, without an OS window:

- Focused resize regression failed before repair (`picker-resize.log`) and passed
  after repair and expanded query/footer checks (`picker-resize-final.log`).
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --locked --offline
  -j2 --features native-image-tests --lib --test choice_picker`: **530 library tests
  passed, two existing skips; eight admission tests passed**
  (`picker-resize-native-final.log`).

Artifacts live under `scratch/agents/root-20260929-m7-resumed/`. This is native
layout and simulated AX-tree evidence, not physical macOS resize, external AX,
IME, process-level idle CPU or visual acceptance. No public API, protocol or GPUI
fork changed in this repair.

Strict all-target native/protocol Clippy with `native-image-tests` and
`-D warnings` also passes (`picker-resize-clippy.log`), along with Rust formatting
and `git diff --check`.


## Managed numeric-step handlers — 2026-10-02

The numeric gallery now uses public `Number_input.run_step` in an existing
page-activation scope. Leaving the page cancels computation and suppresses late
feedback. The helper queues an original-request decline on cancellation or work/
resolution failure, even when all 64 ordinary numeric-command slots are occupied.
A value already dispatched cannot be rolled back by cancellation. Five real-Eio
handler tests and one injected-host App capacity/routing test pass, along with
full OCaml tests/formatting, gallery build and catalog audit. This checkpoint adds
no Rust or protocol changes and opens no OS windows. Physical cancellation/error,
keyboard/IME/AX, resources and release acceptance remain open. Commands and exact
semantics are in the [request contract](../design/number-step-requests.md).


## Slider presentation — 2026-10-02

The numeric page now exposes single/range axis/scale changes and the public
`Slider.Appearance` API: remaining-side single fill, separate rail/thumb/focus
colors, larger targets and geometry. Native TestPlatform checks preserve owner,
focus and values across visual changes, and cancel active capture on target-size
changes. Paired codec, atomic admission, strict lint, full OCaml tests/formatting,
gallery rebuild and catalog audit pass. There are 317 protocol tests, 593 native
library passes (two existing skips) and six slider admission/routing passes.
Animated hover/pressed rings and physical desktop/resource/release qualification
remain open. No OS windows were opened. Exact commands and scope are recorded in
[slider presentation evidence](slider-presentation-och41.md).


## Slider hover/pressed rings — 2026-10-02

Numeric-gallery slider thumbs now have native spring feedback for hover and
captured dragging, with immediate reduced-motion targets and no idle frames or
OCaml animation events. The same native focus/value owner survives. Two solver
checks and a mounted fake-clock test cover retargeting, range-thumb independence,
capture loss, hidden/disabled/read-only/inactive policies, offscreen/transparent
presentation, removal and Close. Full native 596 library tests and six slider
tests pass (two existing skips), as do final focused checks, strict lint, full
OCaml tests/format/gallery rebuild and catalog audit. There were no OS windows.
Physical interaction, GPU visuals, accessibility and measured resource acceptance
remain open; see [exact evidence](slider-presentation-och41.md#native-interaction-rings).

## Navigation and disclosure extensions — 2026-10-02

The Navigation page now demonstrates actual breadcrumb routes and passive members,
compact/full paging, billion-page bounded selection, count shrink/empty data and
disabled controls. Rich accordion headings offer optional/required/multiple
expansion, individual/group disabled state and a native draft retention example.
The public Core/Bonsai/Eio APIs supply these behaviors.

Exact fixtures, deterministic driver tests, native TestPlatform and full local
OCaml/native/lint checks are recorded in [pagination evidence](pagination-chooser-och41.md)
and [disclosure evidence](disclosure-presentation-och41.md). The latest native
library checkpoint is now 614 passing tests with two existing skips, after
connecting native measured disclosure reveal. The page offers an Animate toggle;
Retain paints inert outgoing content, while Unmount disposes immediately and can
animate reopening. Paired fixtures, nine motion-state tests, five layout tests,
two native host tests and atomic admission cover the new behavior. All 319
protocol tests, 22 existing tree/session/disclosure/navigation/allocation checks,
full OCaml tests/format/gallery build, strict lint and catalog audit pass. No OS
windows were opened. Physical gallery qualification, measured resources and
release requirements remain open.


## Calendar retained content — 2026-10-02

The Dates & colors page now uses checked public `View.Calendar_content` slots
for navigation arrows, a month heading and date event badges, in both the inline
calendar and appointment popup. Content updates preserve native draft/focus and
pending confirmation. Native tests caught and fixed duplicate generic child
rendering; hidden progress content stops frame demand and removal releases its
owner. Full OCaml tests/format/gallery build, native 633 passes/two existing
private-D-Bus skips, protocol 326 passes/no skips and strict Rust lint pass.
A fresh installed gallery consumer build passes without launching a window.
The new physical picker walkthrough is authored and syntax-checked but unrun;
physical acceptance and exact viewport observation remain open. See
[contract](../design/calendar-content.md) and
[evidence](calendar-content-och41.md) for precise scope and commands.


## Modal backdrop and accessibility follow-up — 2026-10-02

The Overlays & help page demonstrates public theme-aware modal backdrop colors,
including transparent paint and changes while the same dialog remains open.
A TestPlatform regression reproduced and repaired missing modal accessibility
flags by applying metadata before panel type erasure. Six modal kinds retain
focus/input policy across color changes; popovers remain nonmodal. Full OCaml
checks/gallery build, native 634 passes/two existing private-D-Bus skips, protocol
327 passes/no skips, strict Rust lint and a fresh installed gallery consumer
build pass. The extended physical AXModal/identity walkthrough is authored but
unrun. See the [contract](../design/overlay-backdrop.md),
[evidence](overlay-backdrop-och41.md) and
[22-snapshot feature review](../catalog/overlay-review.md). Real entry-motion and
positioning/inset differences remain open; this does not close overlay-family
or physical milestone acceptance.
