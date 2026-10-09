# A controlled four-page tour with native auto-advance

[artifact_tour.ml](artifact_tour.ml) and [artifact_tour.mli](artifact_tour.mli)
create a carousel of links to Sources, Results, Diagram and Feedback. Selection
belongs to Bonsai; native GPUIO handles gestures, focus and optional deadline
proposals. The tour defaults to manual navigation. No Eio timer or background
page-fetch task is started here.

Use the [isolated environment](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe --full-motion
```

Open Workspace → Take workspace tour. Use arrows/Home/End/numbered controls or
swipe, open a working page from its card, and opt into Start guided tour.
Compare a separate --reduced-motion launch; it pauses auto timing. macOS is the
v1 target; Linux GUI checks remain [informational](../../../docs/platform-release-policy.md).
Every page is local simulation content and no remote media is needed.

## Typed pages and a small request reducer

Read `Page`, `Action`, `apply`, then `component`. Page is Sources/Results/Diagram/
Feedback with stable name-based Carousel.Id values, labels and typed payloads.
Titles/descriptions explain the real workspace destination each card opens;
these are not previews constructed by running those destination controllers.
`Page.items` is the bounded four-item list, not a lazy network collection.

Action is Navigate request, Toggle_auto or Deactivate. `apply` delegates
Navigate to `Carousel.apply_request` against the current model. Toggle_auto
removes an existing policy or creates one with a four-second interval.
Deactivate calls `restart_auto_advance`, invalidating pending automatic proposals
without discarding current manual selection or removing the enabled policy.
Validated literal operations use `Or_error.ok_exn`.

`Bonsai.Cont.state_machine0` initializes a looping carousel with no auto policy;
the first item, Sources, is selected. It returns reactive state plus injected
action effects. `let%arr` combines carousel/inject/theme values into the view.
A reactive value recomputes dependent views when its source changes; actions
are reduced later against the latest model, not performed during view construction.

## Native view, proposals and page content

`View.carousel` receives stable key artifact-tour, label Workspace tour,
`hidden:Unmount`, 360-pixel root height and 308-pixel viewport. It uses
fixed palette styles for viewport/pages/controls and queues requests through
`inject (Navigate request)`. Native arrow/swipe/numbered navigation supplies
requests; `Carousel.apply_request` keeps application selection authoritative.
The selected-label status is derived from that accepted model.

`content item` reads its typed Page payload and creates a marker, title,
description and ordinary Presentation.attachment with an Open <page> button.
`on_open page` is supplied by [Inspector.component](inspector.ml), which maps it
into its route/navigation effects. The attachment is a local composition, not a
file upload/download. `hidden:Unmount` removes inactive native card content;
these cards own no editors/tasks, while carousel model/selection stays retained
in the window's continuously constructed Bonsai graph.

The [Carousel interface](../../../lib/core/carousel.mli) describes native timing:
auto interval begins after settled visible paint, pauses for hover/focus/drag,
inactive or hidden windows/ancestors and reduced motion, then resumes with a full
interval rather than catching up. One automatic proposal waits for an application
revision change. Auto_next includes revision/source/successor; stale proposals
cannot advance an unrelated newer selection, including after looping back to the
same item. Selection/policy updates and restart advance that revision.

## Trace manual navigation, timer delivery and hiding

A native Next request queues Navigate. The reducer applies it to current selection,
Bonsai derives the next page/status and the native view presents its ordinary
transition. Opening Results from a card calls inspector navigation rather than
mutating tour selection. Leaving the page updates reactive `active`; its
`Bonsai.Edge.on_change` callback injects Deactivate when false. Returning therefore
preserves the selected tour stop but invalidates old pending ticks.

Start guided tour injects Toggle_auto, installing the four-second native policy.
After visible settled paint/deadline, native queues an automatic proposal; the
same reducer validates and accepts it. Hover/focus/hidden/reduced motion pause
this native behavior with no OCaml polling fiber. Pause guided tour removes the
policy. An accepted request/revision update is not a statement that every
intermediate transition frame was physically displayed.

The inspector constructs the tour component once per window outside route
selection. Another window has its own carousel selection/policy; window closure
ends state and native presentation. Auto-advance controls only tour selection,
not provider streams, document work or other pages' scopes.

A small adaptation is another local tour card: extend Page.all/name/title/
description, choose a unique stable ID and add Inspector's on_open route mapping.
Keep the collection finite and immutable page payloads free of resource owners.
For another interval use Auto_advance.create and respect its one-second–one-hour
validated bounds. Do not replace native timing with an Eio loop or recreate a
fresh carousel lineage for every ordinary state change.

`python3 scripts/test_agent_chat_tour.py` is the optional macOS input/motion runner;
[README](../README.md)/[existing evidence](../../../docs/evidence/agent-chat-m5.md)
qualify its checks and image-fallback companion. This source review runs no
carousel gesture/deadline or reduced-motion acceptance test.
