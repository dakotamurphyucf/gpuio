# Navigation Lab

Run `./scripts/gpuio exec dune exec examples/navigation/main.exe`.

The public Core/Bonsai/Eio example includes an accessible breadcrumb path,
bounded pagination over a billion-page archive, page-count shrink, a retained
collapsible editor and independently lazy Bonsai content. All data is local; no
network credentials or services are needed.

Page buttons send `Pagination.Request` values to a Bonsai state machine, which
reduces each against the latest model. The buttons do not own another selection.
Localized labels, ordinary native keyboard/AX actions and current-page descriptions
are supplied through `Navigation`. The two breadcrumb ancestors both return to the
archive's first page; the separate route card demonstrates native stack transitions.

The draft's Bonsai computation stays active when its native panel is hidden with
`Content_policy.Retain`. The separate lazy switch changes a Bonsai branch and its
lifecycle hooks. Neither cancels the archive data scope. Window closure cancels the
scope, including its long-lived subscription. Branch deactivation does not imply
Bonsai model eviction, and a native content policy does not cancel an Eio scope.

After building, `_build/default/examples/navigation/main.exe --self-test` opens
one local window, verifies queued page requests and shrink, edits/hides/restores a
Unicode draft, checks hidden focus denial, deactivates/reactivates lazy content,
completes data work while content is hidden, and verifies scoped shutdown. It
closes the window on completion or a reported failure. Native tests separately
cover actual Tab/Enter/AX activation and AppKit description readback.

This example covers implemented navigation/disclosure and modal overlay adapters. Hover cards, carousel and broader acceptance remain part of OCH-37;
Linux GUI validation remains OCH-17.

The lab also includes the initial `Sidebar` composition: grouped nested links,
independent expansion, selected/disabled destinations, a scoped SVG icon, suffix,
header/footer, compact tooltips and command-based context menus. Its external
collapse button remains available in icon and offcanvas modes. Shift-F10 on a
focused destination opens its context menu. Mode changes preserve expansion and
do not reset the main draft. Allocated width animates natively with stable inner
layout and reduced-motion support. Retained offcanvas content slides out after it
becomes inert; Unmount removes children immediately. Run with `--right-sidebar` to place it on the other side.

On macOS, `python3 scripts/test_sidebar.py` exercises the owned application's AX
links/buttons and real context-menu keyboard path, checking current help,
disabled links, independent selection/expansion, icon/offcanvas geometry and
hidden AX content. `--images PATH` optionally captures the three visible states.
It closes/reaps its child on success or failure. This is separate from the public
self-test's reducer/lifecycle checks and from VoiceOver speech validation.

`python3 scripts/test_sidebar.py --motion` launches the lab's `--motion-test` mode
with two-second linear native transitions. It measures intermediate allocated width,
interrupts a collapse, verifies fixed inner width and offcanvas hiding/opening, and
settles a long transition through the application reduced-motion policy. The longer
duration and policy control are test-only; normal usage follows system preference.

Add `--right` to either sidebar test to exercise right-side placement. Motion
checks with `--images PATH` capture an intermediate offcanvas frame: the content
still paints while its AX subtree is already absent. `Style.Inert true` supplies
that distinction; retained buffers survive, focus/pointer/IME input does not.
## Mounted navigation pages

The **Native route transitions** card uses `Gpuio_bonsai.View.navigation_stack`
with an application-owned `Navigation_stack` model and explicit `Retain` policy.
Open the preview, replace its route, and go back to the draft. The draft's native
editor survives while hidden; Rust owns page motion and focus. Replacement removes
the old native page immediately. The existing lazy-computation and Eio data-scope
examples remain independent of route visibility.

`--self-test` additionally checks forward/replacement/back, preserved editor
snapshots and rejection of focus commands to the inactive route. Native tests
separately exercise GPU exit pixels, actual keyboard/AX behavior, disabled focus
fallback, Unmount and disposal. This example is an implementation lab, not complete
OCH-37 acceptance or the final chat showcase.

## Drawers and confirmations

The four drawer buttons open `Gpuio_bonsai.View.sheet` at each window edge. Native
layout clamps the configured extent and keeps focus inside until the app accepts
closure. Escape/backdrop requests close this example's drawer. The **Close with
confirmation** button opens a nested `View.alert_dialog`; **Keep open** is the
first eligible control and receives focus. Clicking its backdrop does nothing.
Escape or **Keep open** returns to the drawer; **Close details** removes both.

The self-test drives this flow through the public Bonsai reducer and verifies
that background editor focus is blocked and its draft survives. Actual native
keyboard/pointer, placement, resize, AX and dismissal routing are checked separately
by `native_controls`. These surfaces unmount on close; their application state and
Eio tasks are not implicitly cancelled by native visibility.
