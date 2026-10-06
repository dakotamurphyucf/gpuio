# Show native loading without a UI timer

[query_loading.ml](query_loading.ml) and [query_loading.mli](query_loading.mli)
provide two stateless views: `spinner ~dark ~label` and `results ~dark`. The caller
already knows whether work is pending. These helpers show that condition without
creating a query, calculating progress or adding an Eio/Bonsai tick task.

From the repository root with the [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe --full-motion
```

Open Results and choose **Slow query** to see the five-second local load, or send
a prompt to see the header spinner. Compare a separate launch with
`--reduced-motion`; indicators stay recognizable while native motion is static.
No provider, network or external asset is needed. macOS is the v1 target;
Linux graphical checks remain [informational](../../../docs/platform-release-policy.md).

## The three concrete indicator kinds

Read the private `indicator` function first. It resolves `Palette.of_dark dark`,
then builds a `Gpuio.Loading.Config` with `kind` and an accessible `label`.
`Or_error.ok_exn` treats the checked-in labels as valid literals; custom input
must respect the [loading contract](../../../lib/core/loading.mli): a nonblank
UTF-8 label without NUL, at most 4096 bytes. `View.loading` receives the validated
configuration and a style rather than an OCaml animation callback.

The base style sets accent foreground, raised background and six-pixel radius.
Additional `properties` supply each indicator's dimensions. `px` constructs
logical-pixel lengths, not physical device pixels. The public loader defaults
to animated, with a 1200 ms native period; this helper does not override either.

`spinner` adds an 18 × 18 size and selects `Loading.Kind.Spinner`, whose native
presentation cycles radial strokes. `results` is a 300-pixel-high, nonshrinking
column with surface/background/border/padding and three children:

- A 180 × 14 `Skeleton` labelled Preparing result columns.
- A full-width × 98 `Shimmer` labelled Preparing result rows.
- Muted text saying Waiting for the local query….

Skeleton pulses a solid placeholder; Shimmer moves a highlight. These are
indeterminate presentations, not a percentage or claim that real columns/rows
are ready. The literal labels and fixed container height are demonstration
layout. [Palette](palette.mli) supplies semantic colors; these helpers own no
asset handles, selections or table cells.

## Follow the actual pending state

[results.ml](results.ml) constructs the Bonsai table graph and obtains
`Pager.value t.pager`. Its `let%arr` combines the current snapshot, busy flag,
query, table output and theme into a view. A reactive value causes its dependents
to update when it changes; `let%arr` derives the view from those values.
`Query_loading` itself simply receives ordinary booleans/strings at that point.

In that caller, `pending` comes from `snapshot.after = Loading`. The footer shows
`spinner` for this paging status, while an empty data source plus pending status
shows the larger `results` placeholder. A separate `busy` flag tracks CPU fixture
construction and shows Building result fixture; it is not the same condition as
pager loading. Failed requests show retry alerts, and completed empty results
show an empty state instead of an indefinite spinner.

For a concrete trace, **Slow query** resets the pager to an empty sample with an
unknown next page. The table's active auto-load requests that page, the pager
publishes Loading, and Bonsai derives the placeholder/footer views. The Eio
loader's five-second delay belongs to Results, not this module. Completion
publishes rows/status and the next derived view removes the placeholder. Changing
the query or resetting cancels obsolete pager delivery. Closing the inspector
hides its native activity while the window-owned query may complete.

In [workspace.ml](workspace.ml), the header mounts `spinner` only while the
conversation is Accepting/Streaming, inside a separate
[Chat_motion.activity](chat_motion.md) wrapper. The spinner has its own native
loading presentation; the wrapper adds the shared opacity pulse. Response
completion/cancel changes observed phase and removes it. No native frame calls
back into OCaml to derive another spinner view. Visibility/unmount/window
lifetime bounds native frame requests, and reduced motion uses a static form.
An input model update or submitted view is not proof of physical paint.

A small adaptation is to add `indicator` options for a slower period or a
caller-selected static policy, validating them through `Loading.Config.create`.
Keep showing it only for real pending state and replace it with retry/empty/
completed content on terminal outcomes. To vary the results placeholder height,
change its container together with its child sizes so it fits the intended pane.
Do not add a timer merely to redraw native loading. The
[view interface](../../../lib/core/view.mli), [Results source](results.ml) and
[README verification map](../README.md#validation) distinguish presentation from
request ownership and actual native test evidence.
