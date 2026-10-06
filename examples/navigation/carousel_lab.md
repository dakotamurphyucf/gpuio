# An application model with native carousel motion

[carousel_lab.ml](carousel_lab.ml) builds Draft, Review and Deliver pages with
native navigation, one draft editor and a shared star counter. The explicit
[interface](carousel_lab.mli) exposes actions, read-only model accessors, a test
observation record and the component factory. Start with `Page`, `Action`,
`Model.apply`, then `component`. The [README](README.md) lists the surrounding
lab and physical drivers; [main.md](main.md) explains startup and diagnostics.

From the repository root, using the isolated
[development toolchain](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/navigation/main.exe
_build/default/examples/navigation/main.exe --carousel
```

There is no separate carousel executable. `--carousel` presents the focused
content while still constructing the enclosing lab model/runtime and data scope.
No file assets or external service are required. Native execution needs a desktop;
see [platform policy](../../docs/platform-release-policy.md) for Linux boundaries.

`Page.t` is a closed variant of Draft, Review and Deliver. `Page.id` converts each
to a validated stable `Carousel.Id.t`; `Page.all` creates ordered labelled items
carrying the variant as payload. Labels are presentation, IDs preserve native
identity across model updates. `Model.t` owns `Page.t Carousel.t`, axis, hidden
content policy, star counter and request count. Initially Draft is selected,
looping is true, axis Horizontal, hidden policy Retain and both counters zero.
The model is abstract in the interface; diagnostics use named accessors.

`Model.apply` reduces `Navigate request` against the current carousel and counts
that request even if disabled/stale requests leave selection unchanged. Toggle
actions evolve axis, looping, disabled flag, native auto-advance policy or lifetime.
Auto-advance config is four seconds when enabled. Trim uses `Carousel.with_items`
with the first two items; removing selected Deliver clamps to Review. Restore
updates with all items. These operations preserve mounted revision lineage;
calling `Carousel.create` again under the same view key would invent a fresh
lineage. `Increment` changes shared Bonsai stars independently of native pages.
See the [carousel contract](../../lib/core/carousel.mli) for bounds and guards.

`B.state_machine0` constructs a persistent graph state machine with `Model.initial`
and reducer `Model.apply`. It returns reactive model and `inject : Action.t ->
unit Effect.t`. Inject creates a deferred action effect, not immediate mutation.
Actions are applied against the latest model, so queued increments or Next
requests compose instead of writing from an old captured selection.
`Input.create` allocates one native draft controller with `initial_draft` seed.
`B.return` supplies fixed config. Rust owns text, caret, IME and undo; the star
counter remains application state.

```ocaml
let%arr model = model
and inject = inject
and editor = editor in
```

`let%arr` reads current ordinary values from reactive inputs to derive a reactive
view. `and` defines dependencies, not threads. `button` returns a GPUIO button
with `inject action`; `toggle` returns a switch with the same action discipline.
`B.Edge.after_display` schedules an effect storing the latest model/inject/editor
in the diagnostic `Observation` ref. That hook is Bonsai/runtime observation,
not proof of physical display.

`UI.carousel` binds the application model to stable key `project-gallery`, label,
axis and `~hidden`. `~on_request` wraps each native request as `Navigate`. The
native renderer handles axis motion, drag, wheel and child editor precedence;
it does not synchronously call the OCaml page builder. `~content` reads an item's
`Page.t`, chooses page text/accent/background and places `Input.view` only in
Draft. All pages display the same reactive star counter. Explicit viewport/page
styles use logical pixels and percentages, with a 248-pixel viewport inside a
296-pixel gallery. Controls share the validated model's available navigation.

Click Next from Draft for a trace. Native input produces a carousel request,
delivered asynchronously to the OCaml UI domain. `inject (Navigate request)`
runs the state-machine reducer; model selection becomes Review. Bonsai derives
new config/help/selection text; GPUIO submits it and Rust owns presentation and
focus changes. Retain hides Draft without destroying its native editor lease,
buffer or history, though hidden focus is blocked. Switch to Unmount, leave Draft
and return: its former native lease becomes stale and a fresh editor uses
`initial_draft`. The Bonsai component/controller and star model stay active;
the workspace data scope is unaffected. Native lifetime is not draft persistence.

Auto-advance is native timing and proposes an action to the same reducer. It
pauses for hover, contained focus, dragging, hidden/inactive windows and reduced
motion; resumption starts a full interval. One pending proposal waits for changed
application revision. The model checks revision/source/successor, so old ticks
cannot override later navigation. No application per-frame timer or catch-up
backlog is introduced. Admission/rendering remain separate from physical display.

The enclosing lab's ordinary `--self-test` covers this component's queued intents,
axis/loop/shrink, retained draft, stale lease after Unmount, initial-text remount,
disabled requests and independent stars/data. Run the full test as shown in
[main.md](main.md); combining it with `--carousel` omits other required lab
placements and is not the documented self-test command. The separate foreground
macOS `python3 scripts/test_carousel.py` checks AX/keyboard paths and a native
auto proposal; `--images PATH` captures settled pages. No new GUI qualification
is implied by this walkthrough.

To add a fourth page, add a variant/ID/label, extend `Page.all` and the exhaustive
content match, and decide whether Trim should still retain only two. Keep IDs
unique and evolve the existing carousel through update APIs. To preserve Draft
across native Unmount, store its text under an application data owner and seed
new placement deliberately; do not mirror every observation into replacement.
The [editor controller](../../lib/eio/text_input.mli) specifies lease/revision
commands. Keep asynchronous I/O in explicit Eio scopes outside `let%arr` and
independent of native page visibility when work must continue while hidden.
