# Collections page implementation

Read [collections_page.ml](collections_page.ml) and its [interface](collections_page.mli).
[pages.ml](pages.ml) routes Collections to `component app searchable window palette graph`.
`searchable` is application-prepared data/service input for the
[searchable list](selectable_preview.md), not created by a render.

`B` aliases `Bonsai.Cont`, `V` `Gpuio_bonsai.View`, `L` its `Virtual_list`, `T` its `Table` and
`Forest` its `Tree` adapter. `graph` hosts reactive computations. `B.state` owns the chosen
`Mode` (initially Messages); state machines own the immutable
[message stream](model/message_stream.md) and local `Grid.t`. They return current models plus
action-injection effect constructors; executed actions reduce against latest state. `let%arr`
reads current reactive values to derive view descriptions; creating an effect does not execute
it.

`Mode` selects Messages, Searchable, Cards, Outline, Results, Structural or Scrollbars. All
child computations are created before mode selection. The final keyed `V.tab_panel` wrappers
activate one view while retaining the others under native panel policy; this is not `match%sub`
removal of the Bonsai computation. Shared [scrollbar descriptions](scrollbar_preview.md)
decorate independent native viewports without sharing offsets.
[Cards](horizontal_list_preview.md) and [structural table](structural_table_preview.md) have
their own companions.

The message list derives its immutable keyed rows from `Messages.rows`, uses estimated height
85, overscan 100 and at most 24 active rows in height 235. A native New message activation
injects Append, the latest message model adds a stable row, the adapter derives the changed
source, and GPUIO reconciles bounded rows/scrolling. Tail following occurs only while the
viewport follows the tail; [Follow.view](model/message_follow.md) derives fade/jump overlays
from observations without owning scroll position. Row renderers start no tasks and allocate no
durable message model.

`outline` is a fully loaded Research/Archive hierarchy wrapped in a `Tree_loading` snapshot.
`Forest.component` owns interactive expansion/selection preferences, seeded Research expanded,
with fixed height 36 and budget 16. Reveal observatory captures a target, opens loaded ancestors
and requests eventual native focus; it does not fetch unknown paths. Selection is read from
`Forest.Output.state`.

`Grid.initial` creates 1,000 integer payload rows and Entry/Category/Detail columns. Entry is
left-pinned, sortable and width 130; other widths are 150/380. Budgets are 24 rows and 72 cells.
`Grid.request` validates accepted resize/move/sort changes, reorders the local data for
descending Entry sort, and formats selection/context/copy/activation notices. Native selection
remains adapter/native-owned and is read through `T.Output.selection`; it is not stored as a
second field in `Grid.t`. Copy notices are proposals, not an application clipboard-write
implementation.

`Grid.presentation` derives colors, stripes, padding and optional grouped headers. `headers` can
supply an independent Inspect button; `render_row_presentation` scopes native state styling,
while `render_cell` supplies both visible text and explicit copy/accessibility text. Native sort
emits `Grid.Request`, updates latest config/data, derives adapter input and reconciles native
order. Select last result uses one controller batch for selection plus reveal, guarded against
retired/reincarnated targets.

GPUIO owns measurement, native focus/scroll and bounded owners; Bonsai owns source/configuration
and retained options. Page destruction retires adapters/row computations; native hiding is not
application task cancellation. Adapt with stable domain IDs, explicit remote sorting/loading
boundaries and durable state outside transient rows. This page does not fetch remote table data
or establish GUI acceptance from its budgets/readouts.

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
