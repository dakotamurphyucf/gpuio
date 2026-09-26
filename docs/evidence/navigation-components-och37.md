# OCH-37 implementation evidence

## Pure application models — 2026-09-25

`Navigation_stack`, `Disclosure` and `Pagination` now compile in the public Core
library. Interfaces were drafted before their implementations. No dependency,
compiler, native protocol or capability advertisement changed in this checkpoint.
These results prove pure policy behavior, not mounted UI or platform acceptance.
The full remaining ticket contract is in [the design](../design/navigation-components.md).

Ten new expect tests pass in the existing `test/view_api` suite:

- Navigation replacement preserves forward history; a new push discards it;
  pop protects the root; stale breadcrumb destinations and duplicate instance
  IDs fail explicitly. Same-ID replacement and explicit payload updates work.
  A complete 128-entry traversal preserves physical identity of caller-owned
  payloads; overflow is rejected and a branch prunes forward entries. Clearing
  does not execute a task-cancellation operation.
- Disclosure handles queued toggles, single/multiple conversion, required-single
  collapse, disabled historical selections, collection replacement/reordering,
  empty collections, invalid IDs and independent nested application models.
  A maximum 4,096-item selection contracts without retaining removed expansion.
- Pagination reduces ordered relative requests against the latest state, ignores
  stale absolute requests after shrink, handles zero pages and growth, and
  separates disabled user actions from explicit programmatic updates. Exhaustive
  totals 1..64 at every current page and all five neighborhood sizes, plus six
  positions in a billion-page domain at those sizes, yield 10,430 passing
  partitions. Every partition covers the exact domain, includes endpoints and
  current page, uses only multi-page gaps, and emits at most 13 items. Invalid
  counts, selections and neighborhood sizes are rejected.

Exact commands, local macOS arm64, isolated repository environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 lib/core/gpuio.cma
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
```

Both exited 0. No graphical windows or long-running processes were started for
this checkpoint. The existing view API expect tests also pass. Native views,
accessibility/focus/IME, hidden mount policy, timers, overlays, public Bonsai/Eio
examples, paired fixtures for subsequent bridge additions, required hosted
macOS/Linux checks and milestone merge are still pending. Linux GUI acceptance
remains deferred to OCH-17; OCH-46 remains the chat showcase follow-up.

## Initial mounted disclosure and hidden-content behavior — 2026-09-26

Core and Bonsai now expose `Content_policy`, `View.panel`, `View.disclosure` and
`View.accordion`. Three additional expect tests verify independent appended-kind
bytes, retained versus unmounted child identities, latest callback delivery,
separate ordered toggle requests and skipped collapsed-content construction under
Unmount. Three Rust integration tests verify strict decoding/truncation/trailing
bytes, atomic rejection of malformed child relationships/labels, repeated
visibility updates and complete tree accounting cleanup.

The independent `disclosure-request.hex` fixture was manually assembled from
bin_prot tags and small integer/string lengths using Python bytes, without either
production codec. It contains an Accordion/Disclosure/Button/Panel tree with kinds
44/43/2/42, a Button(false) control, splices and root publication.

Actual macOS `native_navigation` tests pass:

- Up/Down/Home/End header navigation, disabled-header skipping and wrap, modified
  key exclusion, nested-group independence and exactly one Enter/AX Press event.
- Outer collapse while a nested editor is focused restores the outer trigger;
  hidden children cannot regain focus or take the next Tab. Reopening preserves
  the native editor/focus handle and text. Removing the inner editor restores its
  own trigger and drops the old native entity (verified by a weak liveness probe).
- Real NSView marked-text input enters the nested editor. Once hidden, a subsequent
  native text insertion cannot change it. This is native input-delegate evidence,
  not an IME candidate-window claim.
- A retained focus scope disappears from the active modal stack when its ancestor
  becomes style-hidden. Removal disposes editors/buttons and leaves no pending
  focus repair.
- AppKit expanded state is true for an open header and false after collapse. The
  pinned adapter initially reported false for both, exposing its missing expanded
  getter. The repository-local 0.26.3 patch fixes that behavior. No VoiceOver speech
  or external AX notification-observer acceptance is claimed here.

The existing native controls suite (including focus, overlays, tooltip, commands,
menus and pointer behavior) passes with the patched adapter. Native tabs/retained
panels also pass. Full protocol tests, feature-enabled all-target Clippy with
warnings denied, Core/Bonsai compilation, view API expect tests and formatting
pass. The default OCaml backend and separate static extension-consumer backend
both link with the local adapter; their dependency versions remain unchanged.
A source reconstruction check reapplies the recorded patch and matches every
vendored Rust source exactly; generated backend metadata resolves/tracks the same
local source. Required CI now builds `native_navigation` on both platforms and
runs its macOS functional test. Those hosted jobs have not run for this checkpoint.

Commands (isolated repository environment, local macOS arm64):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test disclosure --test native_navigation --test native_tabs
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_controls
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/extension_consumer/main.exe examples/color_input/picker.exe @test/view_api/runtest @fmt
```

Final post-review checks and commit are recorded in Linear. No new navigation
capability is advertised. Public component examples, broader native appearance,
motion/lifecycle/workloads and the remaining OCH-37 families are still required.

## Breadcrumbs, pagination and current-item semantics — 2026-09-26

`Navigation.breadcrumbs` and `Navigation.pagination` are public stateless Core
compositions; they also accept Bonsai effects directly. Their interfaces were
written first. Five new expect tests cover bounded native descriptions up to a
billion pages, current-page metadata distinct from focus, three queued Next
requests, shrink/clamping/stale-handler rejection, overlapping keyed identity,
whole-model disable without remounting, breadcrumb relabelling/latest callbacks,
inert current/disabled entries and bounded/validated localization functions.
A sixth accessibility test covers the independent current fixture and semantic
placement/description constraints.

The schema appends Navigation role 11 and an optional Current value to semantic
Config. Both codecs match a manually constructed current-page fixture. All six
current tags roundtrip and truncated/unknown/undescribed/field combinations are
rejected. Existing independent field/role/request fixtures include the trailing
option. There is no claim that old and new unreleased binary layouts interoperate.
Existing native form metadata constructors retain their behavior with no current
state. Both languages reject a navigation landmark on text and current state on a
container or form control.

Actual local macOS `native_navigation` passes the existing disclosure suite and:

- AppKit current-page help appears, clears on the old page, and updates to a
  localized description on the new page. Navigation is exposed as a group.
- Selection acknowledgement preserves the focused button's native handle and
  does not move focus to the current page. Tab/Shift-Tab and repeated Enter deliver
  ordered button requests without another native selection model.
- A breadcrumb Link performs one AX Press action. A newly disabled boundary
  control loses focus eligibility. Teardown empties buttons and session accounting.

AccessKit unit tests separately verify `aria_current=Page`, existing Click/Focus
actions, and absent selected/toggled state. The native test proves AppKit help,
not VoiceOver speech or a nonexistent platform current-page attribute.

The new [Navigation Lab](../../examples/navigation/README.md) compiles/links and
its real-window public self-test passes. It edits a Unicode draft, hides/restores
its native panel without losing the editor, denies hidden focus, separately
changes a lazy Bonsai branch, completes an Eio data task while the panel is hidden,
reduces three queued Next requests, clamps count shrink, ignores a stale page
request and cancels its data subscription on window close. Its window exits at
completion. The CI workflow now includes the public check; hosted execution is
still deferred to the consolidated milestone gate.

Commands run in the isolated repository environment on local macOS arm64:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 lib/bonsai/gpuio_bonsai.cma @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --lib semantics::tests --test native_navigation --test accessibility
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol -p gpuio-native --lib --test accessibility
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/navigation/main.exe
_build/default/examples/navigation/main.exe --self-test
```

All pass. The first Cargo command's name filter selects five semantic unit tests
and the harness-free native executable; the accessibility integration test is
filtered there and explicitly runs in the following unfiltered command. That
command passes 202 native unit tests, one native accessibility admission test,
28 protocol unit tests and six protocol accessibility integration tests. The full
protocol suite then passes as well. No dependency versions or navigation capability
bits changed. Required hosted macOS/Linux checks/merge, full Linux GUI validation,
broader layout/appearance acceptance and remaining OCH-37 components stay pending.

## Initial sidebar and independent disclosure headers — 2026-09-26

`Sidebar` now has a bounded immutable model and initial public composition:
grouped nested destinations, separate selection/expansion requests, historical
selection and expansion preservation, Icon/Offcanvas/Never modes, left/right inner
border, validated widths/labels, scoped icons, header/footer/suffix slots, compact
managed tooltips and existing command context menus. It uses explicit native
retention policy and introduces no native selection model or callback during paint.
Width/reveal motion and full sidebar acceptance remain pending.

Six new expect tests pass. They distinguish navigation, expansion and collapse;
reject hidden/disabled/stale/leaf-toggle requests; normalize removed selection and
expansion; accept the 4,096-item/128-group/depth-16 model boundaries and reject
oversized/malformed text and duplicate IDs; preserve retained native link/handler
identity across collapse; fence unmounted handlers; skip unmounted builder callbacks;
and validate dynamic labels/geometry. These model bounds are distinct from native
mounted-workload acceptance. Existing global message/tree/resource quotas apply.

`View.disclosure_with_header` accepts independent header content and a dedicated
Button toggle. Native validation permits a direct Button or a Container ending in
that Button before the Panel. It validates the proposed tree atomically, including
rollback for a missing/nonbutton toggle. Tree lookup is shared by rendering and
focus. Only the toggle has expanded semantics or accordion arrow navigation.

Actual `native_navigation` checks pass independent Link/Toggle activation, mixed
simple/custom header traversal, AppKit expanded state, nested editor retention,
collapse restoration to the dedicated toggle, focused-child unmount while its
panel stays visible, liveness disposal and complete teardown. The latter initially
exposed a focus fallback gap, now fixed with previous focused-child eligibility.
The full native controls suite passes after the change, including modal restoration,
menus, tooltips, commands, progress, toast and pointer behavior.

The public AX test exposed a separate retained-tooltip crash: hidden content had
correctly lost its active focus scope, but rendering still required it. Hidden
rendering now preserves anchor layout/ownership without hover listeners/deferred
surfaces. Hidden overlays also skip deferred rendering; parent popover-anchor
capture is conditional on an active scope. A native regression retains both a
tooltip and popover inside the custom disclosure across hide/show and teardown.
The navigation and full controls suites pass with these fixes. No GPUI/dependency
upgrade was needed.

The expanded Navigation Lab compiles/links and its self-test passes retained
Unicode draft, lazy Bonsai lifecycle and Eio cancellation checks plus sidebar
selection and mode changes with expansion preserved. `scripts/test_sidebar.py`
passes through the owned macOS application's actual AX tree and keyboard delivery:

- Current-destination help and disabled Link semantics.
- Parent navigation without toggling children, independent expand/collapse, and
  navigation to a nested child.
- Expanded width 240 and compact width 56 logical pixels, hidden child/toggle AX
  removal, actionable compact top-level links, offcanvas layout/AX removal and
  restoration of saved expansion.
- Shift-F10 context menu, Enter command invocation and clean window/process close.

Owned-window screenshots of expanded, icon and offcanvas states were captured and
visually inspected locally. They are scratch evidence, not broad theme/density/
responsive acceptance or VoiceOver speech validation. The workflow now includes
the public sidebar AX test; hosted checks have not run yet.

Passing commands, isolated environment, local macOS arm64:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/navigation/main.exe @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test disclosure --test native_navigation --test native_controls
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
_build/default/examples/navigation/main.exe --self-test
python3 scripts/test_sidebar.py --images scratch/agents/root-20260925-resumed/sidebar-images
```

The additional unfiltered native library and `native_tabs` run passes all 202 unit
tests and actual tab keyboard/AX, retained-panel focus/draft/selection checks:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --lib --test native_tabs
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

No new wire tags, capability bits or dependency pins changed. Required hosted
macOS/Linux checks and merge remain pending. Next are native sidebar/navigation
motion and focus, the supplementary overlay variants and carousel, followed by
broader appearance/lifetime/workload acceptance, OCH-38/OCH-39 and the OCH-46 chat
showcase. This checkpoint does not complete OCH-37 or milestone 05.

## Sidebar allocated-width motion — 2026-09-26

`Sidebar.Motion` and `Appearance.motion` now use the existing native animation
wrapper, defaulting to a 200 ms ease-in-out transition. Explicit immediate mode
keeps the same wrapper identity. First placement has no entrance animation;
subsequent widths start at the last painted value. Inner content takes its target
width immediately, while the clipped outer allocation interpolates. This adds no
wire configuration, timers in OCaml or dependency changes.

One additional expect test checks duration bounds (0..10 seconds), no explicit
initial values, unchanged-render stability, matching native owner across widths
56/0/240 and immediate-mode updates. The view API suite, Bonsai compilation,
public example link and Dune format checks pass locally.

The public `scripts/test_sidebar.py --motion` check passes on local macOS arm64.
It uses a two-second linear transition and real AX position readback of adjacent
content. Samples show intermediate allocated widths while the inner sidebar has
already reached 56 pixels, reversal before completion without a target snap,
offcanvas allocation reaching zero while content is inaccessible, and intermediate
reopening geometry. Application Reduce settles the two-second target inside a
900 ms observation bound (the passing run observed the endpoint on its first
sample). The normal public sidebar AX/keyboard/context-menu regression also passes.
Both scripts own, close and reap their test application.

The existing native animation regression initially failed twice at its immediate
whole-window idle assertion, observing render count 22 -> 23. Test-only instrumentation
now counts actual animation frame requests. Across two 180 ms observation intervals,
that count does not advance and no deadline is active; one queued redraw drains
in the first interval, and the second requires strictly unchanged whole-window
render count. The complete native animation suite passes with that stronger
separation of scheduling evidence, including interruption, hidden/reduced idle,
repetition, delayed disposal and window close. No production scheduling behavior
was changed. This establishes absence of ongoing animation work, not the precise
source of an already queued GPUI/platform redraw.

Commands (repository isolated toolchain):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest lib/bonsai/gpuio_bonsai.cma examples/navigation/main.exe @fmt
python3 scripts/test_sidebar.py --motion
python3 scripts/test_sidebar.py
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_animation
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
_build/default/examples/navigation/main.exe --self-test
```

**Still pending:** offcanvas content currently hides immediately during shrinking
allocation. Keeping one outgoing painted visual while denying all interaction is
not yet implemented, and cannot be claimed from these width tests. Mounted
navigation transitions need that same paint/interaction separation. Broader sidebar
appearance/workload acceptance, remaining OCH-37 components and consolidated hosted
macOS/Linux gates/merge remain required. A new hosted public-motion step is wired
but has not run. Full Linux GUI validation remains OCH-17.

## Inert retained presentation and offcanvas exit — 2026-09-26

`Style.Inert` adds base-only portable field tag 65, with a manually specified
request fixture shared by independent OCaml and Rust encoders/decoders. Old field
tags do not move; rebuild both bridge halves together. Core rejects state variants
and supports normal base replacement. Native decoding rejects malformed Boolean,
truncation and trailing bytes; tree state-style rejection is atomic. The full
protocol/native library suites and view API expect suite pass locally.

The native wrapper delegates exact layout and paint, hides the AX subtree and
places a blocking hitbox after descendants. Existing ancestor eligibility denies
focus, pointer routing and native editing, retires popup scopes and suspends nested
motion. A descendant `Inert false` cannot escape its ancestor; clearing the ancestor
restores existing native resources. This adds no GPUI fork or screenshot readback
in production. Application Eio/Bonsai lifetimes remain independent.

Actual local macOS `native_navigation` passes with GPU readback enabled:

- The inert panel keeps its geometry and exact background pixels.
- A retained Unicode editor keeps its native focus identity and text; an actual
  NSTextInputClient marked-text session cannot receive new native text while inert.
- Programmatic focus is denied, clicks cannot activate its button/editor, AX
  button/input objects are absent, and an unrelated external button still works.
- Clearing inert restores keyboard activation and the same editor; teardown leaves
  no editor/button owners or retained session bytes.
- Retained tooltip/popover scopes inside an inert custom disclosure retire safely
  and restore on reactivation; collapse restores the dedicated eligible trigger.

The full native controls and animation suites pass. The latter separately checks
that a repeating animation under an inert ancestor requests no frames and owns no
deadline, then resumes after the gate clears. Feature-enabled all-target Clippy,
Dune and Rust formatting checks pass as recorded in the checkpoint.

Sidebar Retain now keeps its fixed-width content painted and inert while its outer
allocation slides to zero. Left/right clip alignment moves content toward the
corresponding window edge. Unmount still removes children immediately; it does not
silently retain native leases until an endpoint. The public motion test passes on
both sides, measuring intermediate allocations, interruption, zero-width endpoints,
reopening and reduced-motion settling. Intermediate screenshots were captured and
visually inspected on both sides, after AX confirmed the sidebar was absent; they
show the retained content still painting as it exits. These are local screenshots,
not broad theme/density acceptance. The right-side normal keyboard/AX/context-menu
check also passes. Its first run found an ambiguous test selector (the earlier
breadcrumb also says Archive), fixed by searching within the Sidebar landmark.

Commands, isolated toolchain, local macOS arm64:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/navigation/main.exe @test/view_api/runtest lib/bonsai/gpuio_bonsai.cma @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol -p gpuio-native --features native-image-tests --lib --test inert --test disclosure
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_navigation --test native_controls --test native_animation
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_editor --test native_controls --test native_navigation
python3 scripts/test_sidebar.py --motion --images scratch/agents/root-20260925-resumed/sidebar-exit-left
python3 scripts/test_sidebar.py --motion --right --images scratch/agents/root-20260925-resumed/sidebar-exit-right
python3 scripts/test_sidebar.py --right
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
```

The public Navigation Lab lifecycle self-test also passes after the final native
link, including the retained draft, lazy Bonsai branch and Eio scope cleanup.

A further real textarea selection-drag test reproduced continued idle work after
inert blur: whole-window renders advanced 141 -> 151 during 180 ms without mouse-up.
The existing vendored GPUI Base blur path now stops auto-scroll and transient drag
state while preserving the selected range. The regression establishes that the
timer was active, applies inert, drains queued paint and requires no new renders
before mouse-up, with the selected range preserved. It passes with the patch. `third_party/patches/gpui-base.patch`
and its `sources.json` digest were updated, without changing source versions.
`vendor_gpui_base.py --output scratch/agents/root-20260925-resumed/gpui-base-inert-verified-final`
reconstructed successfully from the checksum-verified archive; recursive comparison
matches the committed vendor tree when excluding its local Cargo.lock artifact.

Hosted native-navigation now enables GPU readback; hosted checks have not run.
Mounted navigation stack/transitions, remaining overlays/carousel, broader sidebar
workload/appearance acceptance and consolidated gates/merge remain incomplete.
This supersedes the preceding checkpoint's missing offcanvas painted-exit slice;
it does not complete OCH-37 or milestone 05. Full Linux GUI validation stays OCH-17.
## Navigation transition-state checkpoint (2026-09-26)

The presenter contract is now specified in the navigation design. Its deterministic
Rust motion state and OCaml `Navigation_stack.Motion` policy are implemented, with
an independently specified configuration fixture (`01020101fec800`: selected index
2, retained content, slide, 200 ms). The bounded decoder rejects invalid tags,
booleans, indices/durations, truncated input and trailing bytes. Selection must
also agree with admitted child count; that transaction-level wiring is pending.

Seven transition-state tests cover first placement, push/pop, painted reversal,
speculative/stale paint, third-destination interruption, replacement/removal,
Unmount, policy settling, atomic invalid updates, unchanged-selection updates,
history insertion and 9,999 interruptions across the maximum 128-entry history.
State owns geometry and IDs only; it does not preserve removed page resources or
allocate a second native route history.

Local macOS arm64 commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native -p gpuio-protocol --lib --test navigation_stack
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native -p gpuio-protocol --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @fmt
```

Both pass: 209 native unit tests, 28 protocol unit tests, the independent Rust
configuration fixture/malformed-input test, and the OCaml View API expect suite.
Clippy and the formatter check also pass.
OCaml tests also verify Unmount preserves history positions, empty history, and
duration bounds/rounding. An initial test helper used u32 where NodeId requires
i64; it was corrected. The first expect run differed only in multiline whitespace;
the actual output was reviewed and the expectation corrected.

This checkpoint does **not** establish a mounted navigation-stack widget. The
View/transaction adapter, native rendering, focus restoration, public example and
actual GPU/keyboard/IME/AX scenarios remain to be connected and validated. Existing
sidebar/disclosure native evidence below is separate; no navigation-stack native
GUI acceptance or new capability advertisement is claimed by these unit tests.
## Mounted navigation presenter (2026-09-26)

`View.navigation_stack` and its Bonsai specialization now connect application
history to the native transition state. Retain preserves keyed page descendants;
Unmount skips inactive content builders and immediately removes their descendants.
The protocol appends Kind 45 and operation 48. Independent envelope/configuration
fixtures agree in OCaml and Rust. Native admission checks selected indices,
128-page bounds, direct Panel children, labels and inactive Unmount content, with
atomic rejection and complete accounting cleanup. Source pins and capability
advertisement are unchanged.

The native renderer uses assigned-width offsets and clips page geometry. It builds
hidden retained controls to preserve native identities, paints at most one inert
outgoing page, and retires removed pages immediately. Focus gating follows admitted
selected-page ancestry. Weak remembered controls are pruned with page membership;
return navigation restores eligible focus or selects the first eligible control.
Restoration waits for actual painted controls and never starts an idle poll.

Local macOS arm64 evidence:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native -p gpuio-protocol --features gpuio-native/native-image-tests --lib --test navigation_stack --test native_navigation
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/navigation/main.exe
_build/default/examples/navigation/main.exe --self-test
```

Pass: 209 native unit tests, 28 protocol unit tests, two native admission tests,
two independent protocol tests and the Core expect suite. The actual macOS
navigation window verifies first/destination/back focus, native Enter activation,
disabled remembered-control fallback, inactive editor focus denial, preserved
editor/control identity, actual AppKit AX hiding, replacement/removal/Unmount,
idle redraw counts and zero retained accounting. With `native-image-tests`, GPU
readback finds the exact outgoing pink and incoming blue pixels during the same
slow transition. No screenshot cache or OCaml frame callback implements it.

Expanded native checks also pass hidden and reduced-motion idle, and disposal
during an active run. The final command was:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_navigation
```

The shared-focus editor and controls suites pass (actual keyboard, clipboard,
marked text, modal/overlay behavior and AX regressions). Their combined invocation
subsequently failed only in the newly added navigation reduced-motion test: this
controls harness does not install the application's motion-preference watcher.
The test now sets GPUI's reduced-motion input directly; the final navigation run
passes. The existing animation suite separately validates the application/OS
preference watcher; these navigation results do not revalidate that watcher.

An earlier focus assertion ran with a 420-pixel viewport in a 400-pixel test window;
probe evidence showed the incoming controls were still fully clipped. The fixture
now fits the window, and restoration remains pending until eligible controls
actually paint. Initial test compilation also caught Fill versus Color in the
fixture and typed error handling in the public example; both were corrected.
These are resolved implementation/test findings, not outstanding platform blockers.

The public Navigation Lab's new route card and self-test cover forward,
replacement/back, retained draft snapshots and hidden-route focus denial, alongside
the existing independent lazy Bonsai and Eio data lifetimes. This is initial mounted
acceptance. Nested navigation/modal interactions, maximum mounted-history workload,
resize/layout breadth and broader OCH-37 acceptance remain; no Linux GUI or hosted
CI acceptance is claimed here.

All-target native/protocol Clippy with `native-image-tests` and `-D warnings`, Dune
formatting, rustfmt and the public right-sidebar AX/context-menu regression also
pass. All owned test windows exited. The latter verifies that adding the route
card did not break the existing sidebar flows; it is not a separate route AX test.
## Nested navigation, input isolation and mounted-history workload (2026-09-26)

The expanded actual-window test found and fixed two focus defects:

- A route selected behind a separate modal lost its pending destination-focus
  request. The modal correctly kept focus, but closing it did not focus the new
  route. The request now survives temporary modal precedence while the page is
  selected/visible and is retired if the page becomes hidden or removed.
- With no modal trap, native Tab traversal could visit a still-painted outgoing
  control. The manager now uses its eligible painted order whenever ineligible
  controls are present; ordinary native traversal remains for fully eligible
  frames. Actual forward/reverse Tab and pointer tests verify the outgoing page
  cannot receive focus or activation while its pixels remain visible.

The nested fixture independently changes inner/outer histories, enters marked
text through macOS's native text client, hides the inner editor through outer
navigation, and verifies later input cannot mutate it. Returning restores the
same editor handle. A separate modal retains focus across route changes and
Escape only requests dismissal; accepted close restores the current route.
Reparenting that modal into a page and leaving the page retires its trap and
prevents hidden Escape dismissal events.

The maximum mounted-history fixture contains 128 pages and buttons, four native
editor pages and read-only text elsewhere. It traverses every page with actual
Enter activation, preserving all native handles and constant retained accounting.
Per-page weak focus records stay bounded by 128 and pending destination requests
by one. It resizes during a slide to 300×180, 500×320 and 400×280 logical pixels;
incoming/outgoing offsets stay one assigned width apart, and GPU readback confirms
the selected content fills the clipped viewport. Switching to Unmount releases
127 inactive pages' descendants; final disposal clears editor/button/navigation
owners, focus records and retained session accounting.

An initial 128-editor proposal hit the **existing** 8 MiB/editor reservation against
the 64 MiB/window logical quota. The final test explicitly verifies atomic rejection
before running the mixed-content workload. No quota was changed. A 128-entry
history is not a promise of 128 simultaneous native editors, and the reservations
are not allocator/RSS measurements. The public interface and design now say so.

Final local macOS arm64 command:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_navigation
```

Pass markers include `GPUIO_NAVIGATION_NESTED_OK` and
`GPUIO_NAVIGATION_WORKLOAD_OK`. The final measured traversal had a median of
16,470 µs and p95 of 18,502 µs from transaction application through the test's
two-frame wait. This includes display scheduling and excludes the OCaml/FFI path;
it is a local workload observation, not a render-CPU or end-to-end latency claim.

Shared native editor and controls regressions also pass with the focus fixes.
The combined run subsequently found a stale assertion in the extended navigation
test: clicking the incoming action correctly made it the page's remembered target,
so return focus should be that button, not the earlier editor. The assertion now
matches the explicit user interaction, and the final navigation run passes.
All-target Clippy with `native-image-tests` and denied warnings passes. No source,
protocol or dependency pin changed. Remaining OCH-37 component families and hosted
gates remain open; these results do not claim Linux GUI acceptance.

The public Navigation Lab was rebuilt against the updated native library and its
`--self-test` passes. Dune formatting, rustfmt and whitespace checks pass. All
owned test applications exited.
