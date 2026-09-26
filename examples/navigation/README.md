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
archive's first page; full navigation-stack transitions are still being implemented.

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

This example covers the implemented navigation/disclosure subset. The sidebar,
navigation transitions, overlay variants and carousel remain part of OCH-37;
Linux GUI validation remains OCH-17.
