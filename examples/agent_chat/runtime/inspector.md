# Separate artifact page visits from retained component state

[inspector.ml](inspector.ml) and [inspector.mli](inspector.mli) compose the
workspace's artifact pane: review, diagram, sources, results, feedback, tour and
stage details. Visit history/visibility belong here; each child's data/native
resources retain their own window-scoped ownership. A historical visit is not a
new instance of the source loader, review model or canvas controller.

From the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Explore workspace opens the initial Review page. Workspace/Home returns to its
overview; navigate among artifacts, use Back/Forward/breadcrumbs, open a diagram
stage and Next stage, then close/reopen the inspector. All artifact work is
simulated. macOS is the v1 target; Linux GUI checks remain
[informational](../../../docs/platform-release-policy.md).

## Routes, visits and bounds

Read Route, `entry`, `create`, `toggle`, `change`, `navigate`, `navigation`, then
`component`. Route is Overview | Diagram | Review | Feedback | Tour | Sources |
Results | Stage Stage.t. Entry labels describe routes; entry IDs are decimal
serials for **visits**, allowing repeated visits to the same route without
colliding. `t` stores observable opened/routes, one Diagram controller and a
mutable next-visit serial. Initial opened is false, with history Overview(0),
Review(1); Navigation_stack.create defaults current to its last entry.

`change` returns a deferred effect that transforms the latest route model.
`navigate` does nothing when current route already equals the destination.
Otherwise it increments the serial and pushes a visit or replaces current when
`~replace:true`. At 32 entries a new push first starts again from singleton
Overview, bounding retained history instead of rejecting navigation. That history
reset does not recreate the child application controllers. See
[Navigation_stack](../../../lib/core/navigation_stack.mli) for back/forward/
replacement and distinct native transition ownership.

`navigation` maps current route to [Artifact_sidebar](artifact_sidebar.md), with
Stage visits selecting Diagram. Sidebar selection effects set opened true and
navigate; expansion/collapse changes sidebar preferences without owning routes.
This sharing prevents sidebar and inspector Back/Forward from diverging.

## Build retained graphs and active hints

`component` constructs the review, diagram, sources, results, feedback and tour
Bonsai computations **before** its final page-selection/view expression.
`B.map2` derives active hints from opened/current route; only the matching child
is active. A reactive value updates dependent computations when state changes;
`let%arr ... and ... in` combines current observations/views into a derived view.
Constructing a child graph is distinct from keeping all its native views mounted.

- [Review](review.md) keeps observed progress in its graph while native extension
  content unmounts.
- [Diagram](diagram.md) registers a scene lazily when active and retains it until
  window scope ends.
- [Sources](sources.md)/[Results](results.md) gate automatic loading with active
  while window-owned data/jobs remain independent of visible rows/pages.
- [Feedback](review_feedback.md) closes transient contributor preview when inactive
  but retains rating/note/guidance state.
- [Tour](artifact_tour.md) invalidates automatic proposals when inactive while
  preserving its selected card/policy.

`context_expanded` is a separate Bonsai.state_machine0 Boolean initially false,
with a unit action toggling it. It belongs to this inspector, so it is shared by
its Stage visits rather than newly allocated for every stage/history entry.
No new Eio producer or image registration is created merely to build a route.
Application startup already supplied source/results controller capabilities.

## Page rendering and breadcrumbs

The derived `page entry` uses a normal OCaml match over typed route payloads.
Overview links call navigate and use [Chat_motion.destination](chat_motion.md).
Review includes the checkpoint component and Feedback link. Stage displays name/
description/simulation disclaimer, context toggle/reveal, Next stage and review/
feedback links. Next stage passes `~replace:true`, changing that current detail
visit instead of pushing another visit for each cyclic stage advance.

Breadcrumbs include back entries plus current. A path of up to three visits
shows all; longer paths show root, disabled ellipsis and last two visits (at most
four visible items). Each Choice ID is the visit ID, not the route label.
Navigation's pop_to is applied to latest history; absent/forward-only stale
requests are ignored. Back/Forward/Home similarly transform the latest model;
they do not construct new child state.

The outer View.panel has stable artifact-inspector key, label Artifact workspace,
`active:opened`, `hidden:Unmount` and full assigned pane size. Native close
removes its content/semantics while retaining the application/Bonsai owners.
Header/navigation use [Responsive.at_width](responsive.md) at 420/400 pixels.
`View.navigation_stack ~hidden:Unmount` presents the current history entry with
scrollable page content and the public default native transitions. It receives
already-built child views, not a callback that starts a provider job. Native
transitions/paint do not run OCaml timing callbacks.

## Trace a detail visit and reopening

A native canvas Activated observation calls Diagram's supplied on_open effect.
`navigate (Stage stage)` adds a uniquely identified visit; reactive routes update,
Diagram active becomes false and navigation_stack presents detail content.
The scene registration/geometry remains retained in Diagram. Toggle context
injects the Boolean action; Bonsai derives a new expansion target and native
spring retargets. Next stage replaces the current detail visit; Back returns to
its previous page while Forward can revisit its history.

Close workspace inspector sets opened false; active hints deactivate presentation
activities, and native panel content unmounts. Reopening restores the current
route and existing child state, with native widgets mounted from accepted values.
It does not preserve every destroyed widget's internal undo/focus state; feedback
note snapshots/review properties/diagram viewport are the explicit remount
mechanisms described in their guides. Conversation drafts remain outside this
pane. Window closure ends all window-owned state/scopes. Accepted routes or frame
transactions are not physical native input/presentation acceptance evidence.

A small adaptation is a new pure route: extend Route/label, sidebar mappings,
page renderer and active-hint matches exhaustively. If it owns async data,
construct its controller in the window factory and choose scope before rendering;
never assume popping a visit cancels its task. Keep bounded visit IDs/history and
stale breadcrumb behavior. The [README](../README.md) lists actual native routing/
retention checks and [existing evidence](../../../docs/evidence/agent-chat-m5.md);
this source review starts no GUI run.
