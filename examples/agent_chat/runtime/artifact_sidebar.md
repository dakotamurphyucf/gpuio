# Navigate artifact destinations without owning their tasks

[artifact_sidebar.ml](artifact_sidebar.ml) and
[artifact_sidebar.mli](artifact_sidebar.mli) build the grouped destination links
beneath conversation navigation. Bonsai stores expansion/collapse preferences;
the inspector's current route determines selected destination. Expanding a group
or hiding links does not navigate, cancel a query or destroy an artifact model.

From the repository root with [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Select Run diagram, expand it for Checkpoints/Run feedback, expand Source
collection for Run findings, and try Collapse destinations/Hide mode. Back/
Forward in the inspector updates the current destination. macOS is the v1 target;
Linux GUI qualification is [informational](../../../docs/platform-release-policy.md).
No external data or files are loaded by this sidebar itself.

## Model, IDs and distinct actions

Read `Destination`, `initial`, `Action`, then `component`. Destination is the
closed variant Overview/Diagram/Review/Feedback/Tour/Sources/Results. `name` gives
labels and `id` validates them through `Sidebar.Id.of_string`; `find` uses typed
ID equality to reverse-map a request. Labels serve as fixed fixture IDs here;
localizing them would require choosing separate stable IDs to preserve identity.

`initial` constructs ARTIFACTS (overview, diagram with review/feedback children,
tour) and CONTEXT (sources with results child). Each item has a full label and
short compact fallback. The model has no selected destination initially and
Icon collapse policy. Default collapse/expansion state comes from
[Sidebar.create](../../../lib/core/sidebar.mli), not a second navigation model.

`Bonsai.Cont.state_machine0` returns the reactive model and action-injection
effect. Its Action is Request of Sidebar.Request or Mode of Sidebar.Collapse.
Mode changes only collapse policy. Request reduces through `Sidebar.apply_request`
against the latest model; eligible Select requests schedule `on_select` using
`Bonsai.Apply_action_context.schedule_event`. It checks visibility/enabled state
before that scheduling. Toggle and Toggle_collapsed only update sidebar state.
An injected action is deferred; it is not a native callback synchronously running
application navigation during layout.

## Derive route selection and native presentation

`let%arr model = model and current = current and icons = icons and dark = dark
in ...` combines current reactive inputs into a view. The derived
`Sidebar.select model (Some (Destination.id current))` makes display selection
follow the inspector route without changing collapse/expansion preferences or
writing another reducer action on every render. A selected child may remain
selected even when its parent is collapsed; selection does not force expansion.

`Sidebar.Labels` supplies Workspace destinations navigation, Current workspace
page, Show/Collapse destinations and per-item Expand/Collapse labels.
`Sidebar.toggle` requests collapsed-state changes. The separate Hide mode/Icon
mode button toggles Icon/Offcanvas **policy**, rather than directly toggling the
collapsed Boolean. The same retained collapsed preference can therefore appear
as compact icons or hidden offcanvas content.

`Sidebar.view ~hidden:Retain` uses a 174-pixel normal/44-pixel compact appearance,
semantic palette styles and current accent styling. Decorations map each typed
destination to a registered [Icons](icons.md) SVG: Command/Code/Check/Message/
Spark/Paperclip/Search. Before registration, a missing decoration is None;
controls still have their native accessible names. Native sidebar keyboard,
focus/link/disclosure behavior belongs to the public helper. It does not own
page tasks, source/results data or document registrations.

## Trace selection and expansion

In [Inspector.navigation](inspector.ml), the current route is mapped into a
Destination; a Stage route maps to Diagram. `on_select` maps the destination back
to a route and returns effects that open the inspector and call `navigate`.
A native Run findings activation therefore queues Select, the state reducer
accepts it/schedules navigation, inspector history changes, and the derived
sidebar current selection follows that accepted route. Its source/results
controller lifetimes remain window-owned.

Expanding Source collection instead queues Toggle, changing branch visibility
without calling on_select. Collapsing the sidebar changes presentation and
retains branch preferences; showing it restores them. Inspector Back/Forward
changes `current` directly through route observation, so this sidebar does not
invent another visit history. Both graph/preferences belong to one window;
closing another window has no effect. Native hidden content is not input-active,
while Retain keeps its ordinary native ownership according to Sidebar's contract.
A derived view/accepted route is not proof of a physically presented frame.

A small adaptation is another destination: extend Destination.all/name/find,
place its item in the appropriate hierarchy, add its icon mapping and extend
Inspector's two route mappings. Keep globally unique group/item IDs, explicit
expansion versus selection and route-owned selected state. Adding a link alone
does not create its controller or define its cancellation scope.
The [README](../README.md) and [existing evidence](../../../docs/evidence/agent-chat-m5.md)
link native sidebar/navigation checks; this source review runs no keyboard,
collapse or accessibility acceptance test.
