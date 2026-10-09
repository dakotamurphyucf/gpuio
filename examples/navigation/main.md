# Navigation Lab: three different lifetimes

[main.ml](main.ml) combines archive pagination, sidebar, retained disclosure,
route stack, hover preview, drawer/confirmation and lazy Bonsai content. Local
mock loading shows how application tasks can survive native hiding and graph
branch deactivation. The project gallery is independently explained in
[carousel_lab.md](carousel_lab.md), and asset startup in
[sidebar_icons.md](sidebar_icons.md). Read `Modal`/`Action`/`Model.apply`, graph
setup and final view before the entry point and optional self-test.
[dune](dune) links Core/GPUIO/Bonsai/Eio and Jane Street/Bonsai PPX.

From the repository root, with the isolated
[development toolchain](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/navigation/main.exe
_build/default/examples/navigation/main.exe
_build/default/examples/navigation/main.exe --right-sidebar
_build/default/examples/navigation/main.exe --carousel
```

Launch opens a focused 1120×760 logical-pixel window. There are no network
credentials or external assets: data is mock timer/promise work and sidebar SVG
is embedded. `--right-sidebar` reverses sidebar/content row order. `--carousel`
changes visible main content to the focused gallery; the graph still constructs
the enclosing models/editors. `--motion-test` uses two-second linear sidebar
transitions, forces Full motion initially and exposes Reduce motion control;
it is diagnostic policy, while ordinary launch follows System preference.
See [platform policy](../../docs/platform-release-policy.md) for macOS-first
release and separate Linux build/desktop requirements.

## Application types and latest-model actions

`Modal.t` is Closed, Drawer edge or Confirmation edge. Keeping edge in the
variant prevents contradictory separate drawer/confirmation flags.
`Action.t` describes page/total requests, sidebar/route changes, disclosure/lazy
toggles, preview visibility and drawer decisions. `Model.t` contains validated
`Pagination.t`, `Sidebar.t`, `string Navigation_stack.t` and application flags.
Initially there are a billion pages with page 1 selected, details/lazy enabled,
Archive expanded with Inbox selected, preview/modal closed and inspection false.
Route entries Draft/Preview are created then popped to Draft, retaining Preview
as forward history. IDs use validated constructors; labels do not identify native
resources. `ok` raises for invalid fixed demo config rather than implementing
user-data recovery.

`Model.apply` is a pure reducer. Page requests use `Pagination.apply_request` on
the latest model; shrinking clamps current page and stale absolute pages become
no-ops. Sidebar requests preserve independent expansion/selection. Route actions
pop/forward/replace using stable entries. Replacement creates a new route ID and
native page, rather than editing the old page in place. Modal actions enter a
drawer, wrap it in confirmation, return to that drawer or close both. Preview
requests change controlled state. The [pagination contract](../../lib/core/pagination.mli)
keeps rendered controls bounded (at most 13 items), even for a billion pages;
it does not load a billion rows or start work.

`B.state_machine0` allocates persistent state and returns reactive model plus
`inject : Action.t -> unit Effect.t`. An inject call creates a deferred event;
the reducer applies it to current state when run. Three queued Next effects
therefore move page 1 to 4 without reusing an old rendered page number.
`Input.create` allocates separate retained, route and contributor-note editors;
Rust owns their live text/selection/IME/history. Constant config comes from
`B.return`. The carousel owns a separate state machine/controller.

`let%arr` reads current ordinary values from reactive inputs to derive config,
callbacks or views. Its `and` bindings list dependencies, not threads; it must
not perform I/O or execute effects. `B.Expert.Var.value` exposes external loaded
and icon values as dependencies. `B.Edge.after_display` stores the latest
`Observation` record (model, inject and editor controllers) for the diagnostic;
it is not an actual screen-presentation acknowledgement.

`match%sub lazy_enabled` switches between Bonsai computations. The true branch
allocates its own counter and registers `B.Edge.lifecycle` activation/deactivation
effects updating diagnostic refs; the false branch returns simple text.
Deactivation is distinct from model eviction or native hiding. The retained
disclosure uses ordinary derived children with `Content_policy.Retain`; its
Bonsai graph stays active while native editor focus/input is ineligible.

## Concrete GPUIO composition

`Sidebar.view` binds the application sidebar with retained hidden content,
header/footer callbacks, optional scoped Archive icon, Inbox suffix and a context
menu. The menu references command `sidebar.inspect`; `UI.command_scope` supplies
its registry effect `inject Inspect_sidebar`. Collapse mode changes preserve
expansion. The external `Sidebar.toggle` remains accessible in Icon/Offcanvas
modes. `Style` dimensions are logical pixels/percentages; overflow is in the
main content column. `UI.with_accessibility` names that content as a group.

`Navigation.breadcrumbs` constructs three validated Choice entries; either
ancestor action injects Home. `Navigation.pagination` sends typed requests,
while Shrink/Restore change total through the reducer. `UI.navigation_stack`
uses Retain with `Navigation_stack.Entry` payload content; only Draft places its
route editor. Native page motion/focus stays in Rust. The disclosure uses stable
key `draft` and retained content. These are native placement policies, separate
from lazy branch selection and task scopes.

`UI.hover_card` uses Controlled `model.preview_open`, feeding native visibility
requests back through `Preview_open`. Its trigger button has a no-op click effect;
hover/focus behavior is native. Retained contributor note survives closing while
hidden focus is blocked. `UI.sheet` uses stable workspace key, selected edge and
extent 360, with optional content derived from Modal. A nested `UI.alert_dialog`
uses confirmation content and dismissal returns to Drawer; Keep open is first
eligible control. Close details clears both; modal backdrop behavior follows the
respective widget contract rather than arbitrary view callbacks.

For a trace, press Next archive page. Native button handling delivers the request
asynchronously to OCaml; its callback returns `inject (Page request)`. The
reducer advances current page, Bonsai derives breadcrumb/card/pagination text,
and GPUIO submits the native update. The retained draft is untouched. Collapse
its disclosure: model details changes, native content becomes hidden/ineligible,
but its lease/text/selection survives. Deactivate lazy content: branch hooks run,
while archive work still belongs to its data scope. Neither action implicitly
cancels that scope. Transaction/render acceptance is distinct from physical display.

## Runtime, data work and cancellation

`App.run` owns GPUI on the OS main thread and one OCaml Eio UI domain for startup,
Bonsai and effects. Startup creates external loaded/icon vars, starts
`Sidebar_icons.load`, opens the window, then creates child scope `archive-data`
under the window scope. One mock producer sleeps 0.4 seconds in ordinary launch,
or awaits an explicit promise release during self-test. Its completion effect
sets loaded true on the UI loop. Another producer uses `Eio.Fiber.await_cancel`
with `Exn.protect` cleanup to record data cancellation. It models a long-lived
subscription, not a real service.

Work is owned by data/window scopes, not a transient panel or carousel page.
Window closure cancels the child scope and subscription. Application-scoped
icon loading is independently retired during app shutdown. External cancellation
is preserved and queued scope results are suppressed; read
[Scope](../../lib/eio/scope.mli), [App](../../lib/eio/app.mli) and
[Text_input](../../lib/eio/text_input.mli). Unsaved-draft applications should
install a close decision rather than assume native retention persists after close.

## Optional diagnostic and its limits

After building, run the full lab without focused-carousel/motion flags:

```sh
_build/default/examples/navigation/main.exe --self-test
```

This opens a real window, uses a 20-second timeout and closes on success or
reported task failure. It prints `GPUIO_NAVIGATION_PUBLIC_OK` after verifying
completion, data completion/cancellation and two lazy activation/deactivation
cycles. `await` polls test observations in 5-ms Eio intervals. The local `on_ui`
helper enqueues an owning-UI-domain effect with `Scope.Expert.enqueue` and
`E.Expert.handle`, mapping completion into an `Eio.Promise`. `send` injects an
action through that bridge; `read`/`route_read` explicitly request native editor
snapshots. `equal_retained_editor` reconstructs snapshots with focus false for
typed comparison: retained hiding may change focus while lease, revision, text,
selection and composition must stay identical. These Expert calls/refs/polling
are test instrumentation, not recommended application state architecture.

The test edits a Unicode draft, checks hover open/close focus and retention,
drives drawer/confirmation/background-focus rejection, navigates/replaces/pops
routes with retained editor checks, hides disclosure and checks Focus_blocked,
deactivates lazy content while releasing mock data, queues page intents and
shrinks to 2, ignores page 100, restores retained content/lazy branch, and checks
sidebar collapse/selection/expansion. Carousel scenarios check queued stars,
axis/loop/shrink, retained draft, explicit Unmount old-lease rejection and fresh
seed, disabled requests and independent Bonsai/data lifetime. It also waits for
icon publication before final closure. It does not generate real keyboard, IME
candidate-panel, VoiceOver or physical frame evidence.

`--carousel --self-test` omits editor placements used by that full-lab diagnostic;
use the documented plain self-test command. Separate macOS drivers and optional
capture flags are in the README. Their actual results remain dated evidence,
not automatic acceptance from this guide.

To add a page-associated data fetch, add an action/result to `Model`, start Eio
work in the intended data owner and reject stale completions by request identity.
Keep draft/native identity separate from page number. Add validated total updates
rather than reconstructing models inside `let%arr`; preserve bounded pagination.
Use explicit application persistence when drafts must survive native Unmount,
and keep retained view, Bonsai computation and Eio task lifetimes deliberate.
