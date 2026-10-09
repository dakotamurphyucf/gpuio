# Component Studio: composition around a stable native editor

[main.ml](main.ml) builds local settings and chat/tool-result cards with theme,
validation, loading leaves, avatar source changes and rating. Read graph setup,
the view helpers and compositions, then startup/self-test. Supporting components
have guides: [avatar mode](avatar_mode.md), [avatar assets](avatar_assets.md),
[rating reducer](rating_action.md), and [content fixture](content_cases.md).
[dune](dune) links Core/GPUIO/Bonsai/Eio with Jane Street/Bonsai PPX.

From the repository root with the isolated
[development toolchain](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/presentation/main.exe
_build/default/examples/presentation/main.exe
_build/default/examples/presentation/main.exe --self-test
_build/default/examples/presentation/main.exe --content-check
```

Ordinary launch opens a focused 1040×860 logical-pixel window. --content-check
mounts the separate fixture at width 440. --self-test is for the ordinary studio;
combining both omits its editor/phase/rating observations and can time out.
No external service, asset file or credentials are needed. Startup publishes
embedded avatar sources, and attachment names are fixtures, not file reads/writes.
[Platform policy](../../docs/platform-release-policy.md) separates macOS-first
release from Linux builds/deferred desktop acceptance.

`component` constructs a persistent Bonsai graph. `B.state` owns dark true,
invalid false, Initials avatar mode, avatar status text and local notice;
`B.toggle` owns checked, show-loading and animate-loading, all initially true.
Reactive values represent changing computations; setters/toggles produce effects.
The rating `B.state_machine0` uses [Rating_action.initial/apply](rating_action.md),
so action bursts reduce against latest state. `Input.create` owns a window-bound
editor initially seeded Aster workspace. Rust owns live text, caret, IME and
undo; neither appearance nor error changes replaces it.

`phase` is an external `B.Expert.Var`, initially −1 for ordinary interaction.
Nonnegative diagnostic phases override dark/error/loading/avatar presentation
without overwriting those application state models. `B.Edge.after_display` reads
phase/editor/rating/inject and stores refs used by the diagnostic. It observes
Bonsai processing, not physical screen display. The final `let%arr` reads all
current ordinary values and derives the view; `and` bindings list dependencies,
not parallel threads. No I/O belongs inside this repeated derivation.

`p` chooses concrete `Presentation.Appearance.dark/light`; canvas/ink/muted/accent
are validated colors. These are view appearance values rather than changing the
runtime theme with App.Window.set_theme. Local `action ui_effect ()` adapts an
effect to the action callback form used by these core views. `button` applies
common padding/colors; `heading` creates heading typography. Constructing those
callbacks/effects does not execute them.

`indicator` creates validated Loading.Config for Skeleton, Shimmer or Spinner
with current animated flag and label. `Visibility Hidden` hides loading children
without removing their leaf placements; Static/Animate changes native config.
Animation and reduced-motion response remain native, with no OCaml frame setter.
“Preparing your workspace” is simulated status, not actual background service.
`P.group_box` composes controls plus those leaves.

`Form.Field.create` bundles Workspace name label/help/required plus optional
simulated error. `Form.field` attaches that metadata to `Input.view` and keeps
stable internal control identity as error/help change. It creates no validation
scheduler or text ownership. Another field wraps the response switch. `P.link`
updates a local notice rather than opening a URL. Description entries use stable
keys 0/1/2; badges/tag/status/shortcut labels are presentation components. The
shortcut label is explicitly display-only.

`avatar_config` maps mode plus optional source handles to Avatar.Config with
explicit A fallback and Aster avatar accessibility description. Native leaf
handles missing/loading/failed source and preserves label/identity. `on_avatar_change`
receives Image.State, combines diagnostic ref update with status setter via
`E.Many`, and formats Loading/Ready/Failed. A chosen mode and published source
do not prove decoding. `P.message`/`P.bubble` compose that avatar, local text and
feedback. `View.rating` sends requests to its reducer; read-only/disabled have
separate policies. `P.tool_result`/`P.attachment`/empty-state actions only update
notice. Root View.column/rows use percentage width, logical-pixel spacing and
scroll overflow; Presentation supplies slots and visuals, Bonsai supplies state.

Click Show validation for a trace. Native button handling delivers the callback
asynchronously to the OCaml UI domain, its setter effect changes invalid,
Bonsai derives Field with error and Form.field updates native semantic/visual
associations. The editor's native lease/text/selection remains unchanged. Change
appearance follows the same path with different Presentation/colors. Selecting
Use image derives an asset handle; native decoding reports an asynchronous state,
which updates avatar status via effects and a new derived view. Transaction or
render acknowledgement is separate from physical presentation.

`App.run` owns GPUI on the OS main thread and one OCaml Eio UI domain for startup,
component graphs and effects. Startup creates vars/refs, opens the chosen component,
and calls Avatar_assets.load in application scope. There is no network/file
producer or real assistant stream. Runtime exit on last window cleans scopes,
controllers and sources. See [App](../../lib/eio/app.mli),
[Form](../../lib/core/form.mli), [Avatar](../../lib/core/avatar.mli) and
[editor controller](../../lib/eio/text_input.mli).

## The optional command/render diagnostic

--self-test starts an application-scoped Eio fiber with explicit clock and
20-second timeout. It waits for asset publication; `frame value` sets phase,
waits for after_display observation, and bridges App.Window.request_frame through
an Eio.Promise to obtain a native revision. `read` enqueues a UI effect through
Scope.Expert.enqueue/E.Expert.handle, explicitly reads native editor state and
awaits its reply. These refs/Expert calls/polling are diagnostic infrastructure.

Phases 0/1/2/3 select appearance/error/loading combinations and initials/valid
picture/invalid picture/initials. The test awaits Ready and Failed Invalid_data,
checks increasing acknowledged revisions and exact editor snapshots across
error/appearance/source changes. It enqueues four rating increases to saturate
at 5, then toggles selected 5 to zero, toggles read-only and queues an increase
that must leave zero. Completion effect checks result, sets completed and
force-closes with App.Window.close. Success prints GPUIO_PRESENTATION_PUBLIC_OK.
Scope cancellation suppresses queued completions; callback failure propagates
after cleanup. Read [Scope](../../lib/eio/scope.mli).

This test checks programmatic bridge/render/lifetime behavior, not actual
keyboard, VoiceOver or physical display. The separate foreground macOS
`python3 scripts/test_presentation.py --images scratch/presentation-images-001`
uses AX/keyboard and optional theme captures; the content guide covers its own
driver. Those source instructions do not add platform acceptance.

To attach actual validation, derive its result from application policy and show
metadata through Form.field while retaining the controller placement; issue
explicit guarded commands only for intentional resets. For async work, pass Eio
capabilities and choose cancellation scope intentionally, keeping stale result
identity separate from avatar/native handles. Preserve action-polymorphic slot
composition and validated labels. Add another rating maximum consistently in its
reducer helper, and keep application mock notices distinct from accepted sends.
