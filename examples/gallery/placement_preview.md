# Native popup placement from declarative coordinates

[placement_preview.ml](placement_preview.ml) has no adjacent .mli. Its component
is called by [Overlays_page](overlays_page.ml). Read local state/lifecycle,
placement descriptor, controls and popover. `B = Bonsai.Cont` owns reactive choices;
`V = Gpuio_bonsai.View` declares native presentation. Placement values contain no
Bonsai graph, geometry callback, task or OS screen-position request.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section overlay-matrix
```

Choose Overlays and Open placement preview. Adjust corner/margin/fixed mode while
open and resize the window. There is no dedicated --section placement mode;
[README](README.md) records actual native/platform coverage. Commands are documented,
not newly executed evidence.

`B.state` starts with `open_ = false` and `corner = Top_left`; the initial placement is fixed with the smaller margin.
`B.Edge.lifecycle` clears open on departure. `let%arr` derives placement and view from
palette and current state. In fixed mode `Placement.at_point` chooses popup corner
at (560, 400) window-content logical pixels, with margin 8 or 48. It clamps against
current native viewport without flipping. The trigger remains activation/focus-
return anchor but does not determine fixed position. In anchored mode `Placement.create`
uses default Bottom/Start, gap 8 and selected margin; corner state is then unused.
Native layout can flip preferred side and clamp origin according to measured size.
See [Placement](../../lib/core/placement.mli) for finite bounds, client inset and
oversized-content behavior. These are not device/screen coordinates or content
resize constraints.

controls supplies switches and four selected corner buttons inside the optional
popover content. `Overlay.Config` gives label, width 340 and current placement. Some
content opens the view; None closes it. on_dismiss and Close set_open false.
Changing controls re-derives the same popover rather than toggling closed first.
Native layout owns final bounds and focus without an OCaml measurement callback;
[View.popover](../../lib/core/view.mli) defines anchor semantics/focus restoration.
The page owns its Boolean, not a separate native-window handle or Eio scope.

Trace: Bottom right click returns setter effect → corner reactive value changes →
`let%arr` constructs new at_point descriptor → native panel moves/clamps measured
bottom-right corner relative to point → open Boolean remains true. Closing/departure
removes optional native content; preferences in the Bonsai branch remain independent.
For another anchor, retain a direct accessible button and valid placement bounds;
for actual screen-relative positioning, do not reinterpret these logical content
coordinates. Source inspection/compilation alone does not prove pixel/focus behavior.


The focused `overlay-matrix` check observes all four fixed-point corners while
the same panel remains open, across both themes and three application sizes.
The expected right-corner origin subtracts the measured panel width from the
reference point; bottom corners similarly subtract its height. The tested point
fits the viewport in these cases, so this checks corner interpretation and
retention, not arbitrary clamping or resize behavior. See the
[physical results and limits](../../docs/evidence/overlay-macos-och41.md).
