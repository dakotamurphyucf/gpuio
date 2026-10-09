# Native size rules with independent retained branch drafts

[responsive_page.ml](responsive_page.ml) and its [interface](responsive_page.mli)
demonstrate Container_query. Read typed branch IDs/config, branch helper, size/
observation state, three branch computations and final query/readout. `B = Bonsai.Cont`
builds all reactive branch models; `V` declares views; `Input` owns editor leases;
`Q = Container_query` describes pure size predicates for native evaluation.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section responsive
```

Choose Responsive layouts. Enter different drafts/save counts per branch, test
width 479/480 and height 200/230, then switch back. Save only increments a local
counter; it writes no file. [README](README.md) records native/consumer/platform
limits; this walkthrough executes no build or GUI check.

## First matching rule on assigned logical size

Config has default Compact, then Short when height < 230, then Wide when width ≥ 480.
Ordering matters: 600×200 is Short; 480×230 is Wide; 479×300 is Compact. Range maximum
is exclusive and minimum inclusive. These constants are logical pixels independent
of palette text size/device DPI. [Container_query](../../lib/core/container_query.mli)
validates finite nonnegative ranges/typed branch IDs; config construction does not
measure a window or call a Bonsai callback during layout.

width/height `B.state` start at 400/300. Button effects set offered dimensions explicitly;
query sits in full-width horizontal-scroll wrapper but has exact px width/height
and `Shrink 0`. Selected child intrinsic size cannot decide outer query size, avoiding
a layout feedback loop. The native engine chooses branch from assigned geometry,
then `on_select` reports paint-confirmed selection. Its state machine stores latest
selection plus saturating observation count. No repeat notification occurs for a
resize staying in the same branch, so observed dimensions can describe its last
transition rather than every later size. None before layout shows waiting text.

## Three graphs and three native editor placements

branch creates a saturating save counter and one labelled `Single_line` Input
controller seeded Name ideas stay here. Compact/Short arrange editor/button vertically;
Wide inline. Each is constructed once before final `let%arr` with its own graph state,
then all three views are supplied by typed ID to `V.container_query`. There is no
match%sub selecting only one branch and no `shared` controller duplicated among branches.
Missing/duplicate/extra presentation IDs are rejected by the view contract.

[View.container_query](../../lib/core/view.mli) retains hidden branch computations/
editing state while they neither paint nor `receive` native input. Switching width/
height reveals that branch's own draft/counter without resetting others. The
[Input controller](../../lib/eio/text_input.mli) retains native text/selection/history
while its placement remains mounted; rerenders do not apply `initial_text` again.
A container-query branch change differs from leaving the whole gallery page:
Pages match%sub deactivates that page and unmounts all native leases. Chosen size/
Bonsai counts can survive page visit, but this preview does not persist edited drafts
outside destroyed native leases for reseeding them.

## Interaction and adaptation

Trace: at 400×300 type Compact draft and increment save → Width 480 setter updates
reactive width → `let%arr` submits same config/three branch views with new outer size →
native layout selects Wide → paint-confirmed callback updates observation only →
Wide has its own initial/current draft. Width 479 returns to retained Compact input.
Height 200 selects Short even at wide width because its rule comes first.
The observation does not drive which view is provided or perform filesystem work.

For another breakpoint, update ordered typed rules and provide one stable view for
every referenced branch with meaningful parent size. Keep independently edited
presentations on separate controllers; if draft should be `shared`, move accepted
application data outside branches and define explicit edit/remount policy rather
than placing one native controller twice. No Eio producer/scope is created here;
window/widget lifetimes own native editors. Compilation or observed branch text
alone does not prove retained identity/focus/pixel acceptance across transitions.
