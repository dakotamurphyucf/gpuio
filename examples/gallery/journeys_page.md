# Carousels and journeys page implementation

Read [journeys_page.ml](journeys_page.ml) and its [interface](journeys_page.mli).
[pages.ml](pages.ml) routes **Carousels & journeys** to `component window palette graph`. `B` is
`Bonsai.Cont`, `V` `Gpuio_bonsai.View` and `Editor` `Gpuio_eio.Text_input`. `graph` hosts
reactive state/controller computations; `let%arr` reads current models to derive views.

`Chapter.t` defines Imagine/Shape/Share and constructs typed carousel/history IDs. `Slides.t`
holds a looping carousel and independent axis, initially first item/horizontal without auto
advance. `Slides.apply` reduces requests, toggles axis or installs/removes a four-second native
auto policy. `Rail` builds Projects with Orchard/Observatory children plus Archive; initially
Orchard is selected, Projects expanded and collapse mode Icon. Its reducer handles requests,
Icon/Offcanvas mode and branch activation Select_only/Expand/Toggle.

Three `B.state_machine0` computations own slides, history and sidebar. Each returns current
model plus an effect-producing injector; execution reduces against latest model. History is
seeded Imagine as current with Shape/Share in its forward branch. Native Continue journey
activation injects Forward, moves the history model to Shape and makes `let%arr` derive current
label/page for native reconciliation. Constructing the effect does not navigate. Back protects
the root and preserves forward history; this demo never pushes a newly visited route.

`V.carousel` retains hidden pages and a separate native Imagine draft. Native manual/automatic
requests feed `Slides.apply`; automatic deadlines pause during focus/hover/interaction/reduced
motion rather than invoking OCaml per frame. Selection remains application-owned. The
[measured card track](carousel_track_preview.md) is a separate child component with its own
model/editor and qualification limits.

`Sidebar.view` retains hidden native content with width 180/compact width 48. Its decoration
styles Projects and adds passive suffix “02”. Sidebar selection/expansion and history are
deliberately separate state owners in this source: selecting Archive does not navigate the
Chapter history. The “Destination” readout shows sidebar selection, while “Current journey”
shows history. Branch activation wording describes sidebar behavior, not a route loader.

`V.navigation_stack` displays retained pages with independent Imagine note editor. Inactive
native children are inert to input/accessibility; retaining them preserves native buffers while
the page remains mounted. Hiding does not cancel an application Eio task or persist data beyond
destruction. Two editor placements are separate: carousel idea and journey note do not mirror
each other.

Type in both Imagine fields, advance/back through history, switch carousel axis and sidebar
mode, then inspect their independent readouts. GPUIO owns focus, page motion and native editors;
Bonsai owns models/options; adapters own editor leases. No assets, file/network worker or
business workflow exists. Adapt by explicitly linking sidebar requests to your history if
desired, allocating unique IDs per navigation instance and evolving mounted carousel models
rather than reconstructing fresh lineage each render. Preserve durable drafts/tasks at the
application layer.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks run for
this documentation change. The page has no standalone executable or self-test. Compilation does
not establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
