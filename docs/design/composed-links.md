# Composed links and native focus policy

OCH-41/OCH-17; pinned-source review at GPUIO `1785139`.
Status: **native content/focus contract validated locally on macOS; bit 48
(`CAP_LINKS`) now negotiates composed links**. Fresh installed-consumer verification
and release integration are recorded separately below. The dated development
checkpoints retain historical counts and limitations; the current scope is summarized
here and in the final native-focus checkpoint.
Existing `Presentation.link` continues to provide a text-only Link-semantic button.

## Why a native contract is needed

Pinned Longbridge sources:
[base/link](../catalog/sources/base-link.rs.txt) and
[component/link](../catalog/sources/component-link.rs.txt), revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The exact files and upstream license
are checked in and recorded by `docs/catalog/sources/manifest.json`.

| Surface | Existing GPUIO | Required closure |
| --- | --- | --- |
| Text, themed link styling, asynchronous activation | `Presentation.link`, ordinary `Style` states and generation-checked button delivery | Keep these behaviors and current helper compatibility. |
| Arbitrary composed visible content | Buttons only admit two decorative icon slots in both renderer and native tree validation | Add a native Link root and documented passive content policy; do not smuggle unsupported children through icon slots. |
| Target/open callback | Caller's action/effect captures its route or URL; desktop opening remains an explicit job | Preserve this ownership. No URL needs to cross the link transport merely to call back into the application. |
| Activation observer after injected open | Caller composes the application action/effect | Document ordering within that action; native delivery enqueues once and never synchronously calls OCaml. |
| Disabled interaction/style | Existing button helper fences disabled callbacks and native focus | Preserve native pointer/keyboard/AX rejection and dynamic style recovery on the new root. |
| Explicit AX label | Existing text helper permits ordinary accessibility metadata | New composed root requires a validated name, independent of visible content and target. |
| Tab stop/order | Existing helper always uses the ordinary button Tab stop/order | Add explicit native tab-stop policy and signed ordering with correct modal/inert behavior. |
| Native identity/resource retirement | Existing keyed reconciliation and native leases | Content/config changes must retain the root handle; removed descendants release resources and stale actions must be fenced. |

The old styled component stores `disabled` but does not use it, always opens its
href before its callback, and is pointer-only; its own tests document this. GPUIO
will keep its stronger disabled and keyboard behavior. The newer base link already
uses injected navigation and real keyboard/AX activation. It does not set an AX
URL, so no URL attribute parity is implied by its `href` builder.

## Configuration foundation

`Link.Config.create ~label ?disabled ?tab_stop ?tab_index ()` returns a validated,
abstract OCaml value. The label is nonblank according to Core's ASCII whitespace
set, valid UTF-8, without NUL, and at most 4096 bytes. It is an accessible name,
not rendered content. No trimming or normalization changes the caller's text.

Defaults: enabled, Tab participation true, index zero. `tab_index` is a signed
integer in -1,000,000..1,000,000. A negative index is an ordering value, not an
implicit opt-out; `tab_stop=false` explicitly excludes Tab/Shift-Tab while allowing
pointer and explicit accessibility/programmatic focus when enabled. Disabled
suppresses every activation/focus route, regardless of stored tab intent. Keeping
tab intent while disabled allows predictable re-enablement.

`Gpuio_protocol.Link_wire` and Rust `link::Config` have the same field order:
UTF-8 label, disabled Boolean, tab-stop Boolean, signed integer index. Rust's
standalone decoder bounds the declared string length before allocation, validates
Boolean tags and indices, rejects invalid UTF-8, truncation and trailing bytes,
and caps the whole frame at 4116 bytes. OCaml validates constructors and
`Link.Expert.of_wire`; generated `bin_read` alone is not an admission boundary.

Independent fixture: label `Guide 世界`, disabled false, tab-stop false, index -2:

```text
0c477569646520e4b896e7958c0000fffe
```

The original configuration-only checkpoint added no opcode, node kind or
capability. The subsequent development implementation appends Link kind 51 and
Set_link operation 60. That development checkpoint left capabilities unchanged.
With native content and nested-focus validation complete, `CAP_LINKS` is now bit 48,
and its introduction used paired mask `562949953421311`. The current mask,
including subsequent features, is specified in [bridge-v1.md](bridge-v1.md).
Required-mask Hello fixtures in
OCaml and Rust cover both Link alone and the full current feature set. An older
host rejects the new required bit; older operation tags remain unchanged. This
pre-release bridge still carries no general mixed-version compatibility promise.

## Configuration validation checkpoint — 2026-09-29

Three Core expect tests pass: independent bytes/decoded-value validation, default
and all focus-flag/index combinations, malformed labels and numeric boundaries.
The full Rust protocol suite passes **231 tests**, including two new link tests
for independent bytes, every truncated prefix, invalid Boolean/UTF-8 framing,
trailing bytes, exact limits, extreme indices and declared-allocation bounds.
Source manifest/hash auditing and the full local isolated Dune
`@all @runtest @fmt` pass. Strict protocol Clippy (`--all-targets -- -D warnings`)
also passes. These data/build tests are not native link, Tab or AX
acceptance. No GUI window was opened for this configuration-only checkpoint.

## Mounted development checkpoint — 2026-09-29

`View.link` and `Gpuio_bonsai.View.link` now accept validated configuration,
passive children and one asynchronous activation. `Presentation.composed_link`
adds ordinary theme/style defaults; the original text helper stays compatible.
The native root retains its focus handle across content, config and theme changes.
Its config label supplies the accessible name; ordinary explicit accessibility
metadata can refine that name. Routing/URL opening stays in the application effect.

Descendants currently admit layout, text/styled text, image/icon, avatar, loading
and animation nodes, without callbacks. Nested links/controls, selectable text,
scrolling, inert branches and pointer shields are rejected. Selection inherited
from outside the root is suppressed for its content. Content is bounded to 4096
nodes/128 levels, additionally subject to global tree depth, transaction and
retained-memory limits. Resources use the existing scoped registration/lease
ownership. The final native tree revalidates dirty ancestors, including mutations
to child handlers/styles, and rejects invalid batches atomically.

Implemented checks:

* Six OCaml expect tests: independent config/operation/kind bytes, validated
  labels/focus intent, passive-content boundaries, current callback delivery,
  retained root identity, disabled/unmounted stale actions and idle reconciliation.
* Four Rust admission/session tests plus one operation fixture: final-tree
  rollback, forbidden deep mutations, exact width/depth limits, retained payload
  budgets, disabled delivery with a retained handler and stale generations.
* `native_link`: a real macOS window with GPUI-dispatched pointer/Return/Space,
  stable signed Tab order and ties, reverse/dynamic order, Tab opt-out, inherited
  selection suppression, focus-trap entry/restoration, hidden/inert exclusion,
  native identity and disposal. Direct AppKit AX checks verify Link labels,
  explicit focus without activation, one press and disabled focus/action rejection.
* Public `--section links` gallery: **eight theme/description/icon combinations**,
  **34 real pointer/Return/Space/AX activations**, native identity (`CFEqual`),
  forward/reverse order, disabled recovery and zero image/source registrations
  after page departure. The optional SVG and localized descriptions use public
  APIs. Both theme screenshots were produced and inspected.
* Full Dune `@all @runtest @fmt` passed before the subsequent gallery addition;
  the final gallery/link/format build and strict native/protocol all-target Clippy
  pass separately. The existing full native controls suite and slider suite also
  pass, including the latter's 1024-owner/2048-thumb disposal checks. Those slider
  checks do not yet establish mixed slider/link custom ordering; their timings
  are regression diagnostics, not the release performance acceptance workload.

The native regression exposed and fixed a redraw issue: focus records are rebuilt
every paint, but post-update focus finalization is not run every paint. Traversal
now orders its current eligible records on each Tab key, preserving paint order
for equal indices. It never temporarily focuses an ineligible control to inspect it.

### Measured focus and ordinary scroll reveal

The initial public gallery exposed a defect: the fallback Tab order omitted
controls outside the current scroll mask. Native controls now record measured
bounds and their actual paint ancestry. Admission projects the target through
ordinary scroll owners and fixed clips without moving or focusing anything.
A target with no visible portion after clamped projection is excluded from the
fallback order and explicit generic AX focus. A clipped target also disables the
native fast path, so default index-zero traversal cannot reintroduce it.

A newly focused target, Tab request, or explicit generic AX focus request reveals
once after complete paint. Each ordinary scroller moves the least distance on its
actual scroll axes, clamped to native limits. Oversized targets align their leading
edge. Projection intersects the resolved overflow mask, including visible borders,
before passing the visible portion to an outer owner. An owner never scrolls its
own contents to reveal its root focus target. Fixed clips cannot be moved.

Paint ancestry is deliberate: floating deferred dialogs do not inherit the scroll
and clipping boundaries of their retained-tree anchor. Managed lists and tables
retain controller-owned scrolling and focus semantics; this does not materialize
unloaded rows or promise Tab traversal of an entire virtual collection. The frame
records contain weak scroll references, and reveal uses a weak last-focus identity.
There is no new timer and no redraw request when offsets are unchanged. Scrolling
away with the same focus does not continually pull the viewport back.

The public eight-case/34-action gallery test now reveals only its initial link
with wheel input; subsequent real Tab/Shift-Tab steps must make the focused link's
AX bounds fit the named Component preview viewport. It passes without the previous
manual successor-scroll workaround. The native fixture additionally exercises
nested XY scrolls, signed order/ties, range-thumb order inside a trap, explicit AX
reveal without activation, fixed-clip exclusion at zero indices, one-axis limits,
negative unreachable content, oversized targets and idle settling. See the
[focus/reveal evidence](../evidence/focus-reveal-och41.md) for final regression
commands and the precise platform scope.

### Focused non-stops remain traversal anchors

An eligible link with `tab_stop=false` can still be focused through pointer or
explicit accessibility input. Fallback navigation retains that entry while
locating current focus, then searches at most one cycle for the next Tab stop.
Previously, filtering non-stops first lost the anchor and incorrectly jumped to
the first/last stop. The native regression reproduced this before the fix.

This applies to stable signed ordering and tied paint order, including index-zero
fallback used for traps or excluded fixed-clipped controls. A missing anchor
chooses the first/last stop; a scope without stops focuses its root. Traversal
never briefly focuses intermediate records. Changing Tab participation preserves
the current native handle and focus. Native tests cover pointer/AX anchors,
dynamic opt-out, signed/zero/tied indices, forward/reverse wrapping, all-nonstop
traps, one-stop cycles, fixed clipping and neighboring range-slider thumbs.

The pinned base Button/Link/Checkbox/Switch `.tab_group()` calls found during
review are test-harness wrappers, not production component groups. GPUIO's host
does not create native Tab groups; its deferred overlay adapter does not use the
base Popover group. Full docking is post-v1. This corrects the source-review
assumption without claiming untested extension-owned group/path ordering.

### Passive content and lifecycle checkpoint — 2026-09-29

The native Link fixture now mounts an avatar fallback, loading spinner, tween,
animation program and localized text under one root action. The seventh Core
expect test checks their public composition, one callback owner, idle
reconciliation and stale action retirement; observed child animations are rejected.

The macOS `native_link` run with `native-image-tests` passes:

* GPU readback of the avatar fallback glyphs, pointer activation through all four
  passive content types, and a single root Tab target.
* Measured tween/program widths at 0/200/400/600/800 ms, including the loop boundary,
  using existing test clocks and restoring realtime afterward. The loading phase
  advances in realtime without a tree revision change. Reduced motion settles to
  no further renders during the observation interval.
* Exact raw mailbox comparisons: one current root Press per activation and one
  Rendered acknowledgement per painted transaction. Autonomous content emits no
  bridge events; unexpected variants and queue flooding fail the test.
* Native AX name override/reset and AXPress, disable/unmount between mouse-down
  and mouse-up, retired motion owners, and remount with a fresh node/handler/focus
  identity. Updating replacement text retains that identity.

This uses GPUI-dispatched pointer/keyboard input in a foreground macOS window,
direct AppKit accessibility actions and actual GPU readback. It is not VoiceOver,
physical keyboard/IME, Linux GUI or release performance acceptance. No production
code or dependency was changed in that checkpoint (`05f9e0c`). The following
image/style work identified a focus gap in that test's initial conditions.

Reproduction in the isolated checkout:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --test native_link --features native-image-tests --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/link/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
```

Fresh consumer/release checks and remaining content/style/group acceptance remain
open. Native in-flight pointer fencing and Core queued stale-event rejection now
have separate evidence; neither alone implies every pending native action path.

### Image composition and loading-child focus

The added native image fixture mounts an Image and Avatar from one encoded PNM
source, retires its registration before the first paint, and checks actual GPU
pixels. Replacing the shared source changes both interiors while retaining the
link's native focus handle. Measured foreground colors cover root/text inheritance
through base/hover/pressed/focused/disabled states, an explicit avatar override,
and removal of all state refinements. Replacement followed by unmount before the
next paint retires both image owners and all encoded-source charges. Raw events
remain limited to the root action and transaction paint acknowledgements; the
window becomes idle afterward. This does not simulate a deliberately delayed
decoder completion or assert that every decoded cache entry is immediately evicted.

The public rich-preview walkthrough exposed a real focus defect: clicking a
loading child activated its Link but left focus on the previous destination, so
Return then activated that previous link. Loading's standalone mouse-down handler
called `prevent_default`, suppressing the parent's ordinary focus behavior.
Rendering now carries explicit Link-content context through passive descendants
and lets loading mouse-down reach that root. Standalone indicators keep their
existing focus-preserving behavior; disabled Link routing still rejects focus
and actions. No protocol change or fork patch is needed.

The regression starts with a different ordinary button focused before **each**
passive-child click, then checks native focus, a simulated application text update,
and AppKit AX focus. It failed before the renderer change and passes afterward,
including a disabled root's loading child. The earlier sequential child-click
fixture already had the root focused after clicking the avatar; that was
insufficient to establish loading-child focus entry. The native presentation
suite also passes loading cycles/idle/minimize and existing rating/AX behavior.

The gallery now offers `Rich link previews` and `Last opened` feedback using only
public OCaml APIs. The latter lets automation verify the destination in addition
to the shared counter. Image-backed Avatar, Image and Loading replace the SVG
icons while preserving link identity; their assets remain scoped to the page.
The final local macOS walkthrough passes eight theme/content/icon cases plus the
three rich previews, **42 pointer/Return/Space/AX actions**, signed Tab policy and
viewport reveal, disabled recovery, root identity and zero image/source counts
after departure. Rich content stays enabled through departure so the cleanup
check includes the loading and image-backed branches. The rich-preview screenshot
was inspected. This is external AppKit automation, distinct from native fixtures'
GPUI-dispatched input. No Link capability bit or whole-family/release acceptance
is implied by this checkpoint.

Local validation for this renderer/gallery checkpoint: `native_link` and
`native_presentation` with `native-image-tests`; full isolated Dune
`@all @runtest @fmt`, followed by a gallery build/format check after its final
cosmetic adjustment; native/protocol all-target Clippy with `-D warnings`;
catalog source audit and Python parsing. Rust library tests pass 400 native tests
(two private-bus tests require the separate isolated-bus invocation) and 37 protocol
unit tests. The gallery command was
`python3 scripts/test_gallery.py --section links --images <local-artifact-directory>`.
All GUI processes were reaped. These commands used the pinned repository toolchain
on macOS 14.5 arm64; required Linux CI and full release gates remain separate.

### Styled text and an outer highlight scope

`View.styled_text` is passive Link content. A surrounding `View.highlight_scope`
can cover that text, but a scope inside the Link is rejected by the passive-child
contract. The eighth Core expect test exercises this public composition and idle
reconciliation. The native fixture mounts a scope without an observation callback,
a Link with one action and a localized text child with a purple foreground span.

Actual GPU readback verifies purple glyphs over the search wash. Changing only
the wash color retains the matcher epoch; replacing the source removes the old
wash after the new zero-match result arrives. Pointer and Return activate only
the Link, and root identity/native/AX focus survive the source update. Inherited
selection from the outer scope creates no text selection owner inside the Link.
The entire bridge mailbox contains only the expected root actions and transaction
paint acknowledgements. Unmount releases the scope and settles to idle.

The full `native_link` fixture, targeted Core tests/formatting and strict
native/protocol all-target Clippy pass locally for this test-only addition. This
does not add a highlight callback under the Link, advertise a new capability,
or establish extension-owned internal Tab ordering by itself. That ordering was
subsequently verified against the SDK's primary/internal focus contract in the
[nested-extension checkpoint](#nested-extension-focus-correction--macos-validation).

## Full implementation and acceptance contract

1. **Public view and styled helper.** Draft the composed `View.link` and
   `Presentation` interface around `Link.Config` and ordinary view children.
   Keep target routing inside the application effect. Return a typed construction
   error for unsupported content rather than allowing a malformed native tree.
   Preserve the simple existing text helper and its asynchronous effect semantics.
2. **Content ownership and semantics.** The root owns one activation/focus target.
   Support ordinary layout, text/styled text, images and icons as composed content;
   explicitly audit passive animated/loading/avatar/highlight content. Do not add
   nested editors, links, buttons, scroll owners or synchronous callbacks under a
   single semantic link. Specify permitted wrappers, resource leases, selectable
   text suppression, pointer occlusion and child AX exposure. Existing arbitrary
   `AnyElement` upstream composition does not justify an unsafe unrestricted FFI
   subtree. More complex interactive composites keep independent controls.
3. **Native admission and rendering.** Add a dedicated Link node/config operation
   and capability only once the complete path works. Validate kind/config pairing,
   passive descendants and handler eligibility on the final transactional tree,
   including dirty ancestors after Bind/SetStyle/SetImage/SetAccessibility updates.
   Invalid batches roll back atomically. Preserve all existing button icon rules.
   Retain the root's native focus state while updating content/configuration.
4. **Focus and input.** Reuse the native action/focus gate rather than forwarding
   raw OCaml key callbacks. Pointer, Return, Space and AXPress each enqueue once;
   disabled, hidden, inert, stale-generation and modal-ineligible roots enqueue
   nothing. Focus requests do not activate. Tab opt-out remains explicitly
   focusable when eligible. Index changes preserve current focus.
5. **Ordering across focus scopes.** GPUI's tab map orders index paths, whereas
   GPUIO's current modal/inert fallback uses recorded paint order. Merely calling
   `FocusHandle.tab_index` would therefore be incomplete. Define and implement
   consistent ordering for ordinary and gated traversal, preserving paint order
   for ties and existing controls at index zero. Verify mixed controls and native
   compound widgets, Tab/Shift-Tab, modal traps/restoration, hidden/inert branches
   and dynamic order changes without temporarily focusing an ineligible control.
   Native group/path interactions require implementation review and tests before
   freezing the scope semantics in public docs.
6. **Meaningful evidence.** Independent operation fixtures, malformed transaction
   rollback/bounds, OCaml reconciliation/current callback/stale delivery tests,
   native GPU/AX/input/resource tests, and a public gallery card. Use real pointer
   and keyboard input locally; distinguish AX automation from VoiceOver. Test
   content replacement/removal, theme/style changes, disabled/re-enabled roots,
   image lease disposal and idle stability. Then update both link catalog rows.
7. **Release integration.** Full local checks, required macOS/Linux builds/unit/
   private-bus/consumer CI, fresh installed gallery, reviewed PR and final release
   acceptance remain separate gates. Linux desktop qualification stays OCH-47.

This closes neither the link review nor milestone 07 by itself. Other catalog
families, macOS IME/VoiceOver, predeclared performance/resource workloads,
clean-machine distribution and release documentation remain required.

## Nested extension focus correction — macOS validation

A real SDK fixture reproduced a boundary-navigation failure: with signed Link
ordering and a trapped scope, Tab from an extension's last internal control did
not reach the following Link. The host recorded only the primary handle and
looked for exact focus equality, so it treated the internal control as having no
position in the host order. This was a dispatched native-input failure, before
any production correction.

The correction resolves the exact recorded handle first, then its nearest
recorded native ancestor. Ownership is resolved before eligibility, preventing
an eligible outer input region from admitting a disabled inner component. The
same lookup serves traversal, post-update focus preservation, hidden-owner
recovery, modal/route restoration, command routing and component-bounds reveal.
Package navigation and native handle ownership remain unchanged; no wire, SDK
version, dependency or fork change is needed.

`rust/native/src/link_extension_test.rs` adds a package-owned two-button group,
negative/positive neighboring Links, an outer non-stop input region and a trap.
The assertions cover internal/boundary Tab and Shift-Tab, wrap, retained-property
updates, nearest owner, restoration of a child after a nested modal, disabled
eligibility, exact bridge output and idle disposal. The complete native Link
fixture now passes on an unlocked macOS arm64 desktop, including
`GPUIO_LINK_EXTENSION_OK` and the other six result markers. The native controls,
input-region, navigation and menu suites also pass against the shared correction.
This verifies dispatched GPUI input and the fixtures' direct AppKit AX checks;
it does not substitute for real VoiceOver, Linux desktop or release qualification.

Earlier reruns waited for their first rendered frame while macOS reported its
screen locked. Startup markers distinguished that wait from a test assertion
failure. Those runs were terminated and were not counted as passes. The user
closing an earlier window is not an established cause of the reproduced defect.

The local Rust library tests (400 native, 37 protocol; two private-bus tests
excluded from the normal run), strict Clippy and full isolated Dune
`@all @runtest @fmt` pass for the correction. Reproduction: build `native_link`,
`native_controls`, `native_input_region`, `native_navigation` and `native_menus`
with `cargo test -p gpuio-native --features native-image-tests --locked -j2 --no-run`
(select those five `--test` targets), then run each built executable on an unlocked
desktop with a bounded process timeout. Link capability advertisement now follows
this validation. Fresh installed-consumer/catalog acceptance, hosted checks and
OCH-17 release gates are recorded separately and remain distinct from these native
regression results.


## Capability and independent-consumer checkpoint — `586b3ec`

At this checkpoint, composed links advertised bit 48 (`CAP_LINKS`). The shared mask was
`562949953421311`. Independent OCaml/Rust Hello fixtures cover Link alone
(`0001fc0000000000000100`) and all required features
(`0001fcffffffffffff0100`). Session tests reject negative and unknown masks without
entering the ready state, then accept supported negotiation and reject repetition.
Existing operation tags remain unchanged. Subtree highlighting has no advertised
bit in this checkpoint; Link composition evidence does not settle that separate
feature's acceptance.

Local macOS arm64 checks:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --test session --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
```

All pass: 233 protocol tests; 400 native unit and six session tests. Two private-bus
tests are ignored by the normal macOS run and remain required on Linux. The nine
Link expect tests are included in the full Dune check.

A fresh independent gallery stages installed public libraries under its own prefix
and compiles from outside the repository with an independent locked backend. Its
first build exposed a missing toolchain pin: Dune's child Cargo selected the
machine's global Rust 1.88 instead of GPUIO's 1.97.1. The consumer runner now copies
`rust-toolchain.toml` into that workspace; the fresh retry builds successfully
without changing any global default or opam switch. See the
[consumer instructions](../../examples/extension_consumer/README.md).

The focused GUI acceptance is still pending for that installed build. One run
stopped on a pointer-occlusion guard; two subsequent runs lost their gallery
window before completing (after 33 and four successful activations). No product
cause is established. They are **not passes**, and prior repository-gallery/native
checks are not substitutes for this installed-consumer check. The runner now
reports actual/expected owning PIDs and AX statuses on pointer-occlusion failures.
Both source Link rows, combined consumer/CI and OCH-17 release gates remain open.


A subsequent fresh consumer with the border-style addition builds and passes its
focused border gallery. Its Link run identifies Chrome as the covering process
at a pointer-occlusion failure. The reveal helper now raises the gallery before
each desktop-pointer action group, without requesting target-leaf focus; the owner
guard still rejects unrelated windows. The next attempt completes 37 activations,
then loses the gallery window before the 38th observation. This remains an
incomplete Link run, with no established cause for the window's disappearance.
The independent Link acceptance rows remain pending; the passing border check
is not substituted for them.

The later installed consumer used for Empty's successful acceptance also remains
incomplete for Link: a run lost all AX windows while waiting for activation 35
(34 successful activations); the diagnostic retry stopped before a rich-preview
click because Chrome covered that point. The driver now prints the rich target's
rectangle/point and rejects window close/minimize/zoom controls even when they
belong to the same process. No cause for the earlier window disappearance is
established. The gallery's optional `--trace-windows` records future close-request
reasons while preserving normal application decisions. No Link acceptance status
changes on the basis of these interrupted runs.
