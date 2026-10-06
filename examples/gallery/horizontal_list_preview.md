# Variable-size card list walkthrough

Read [horizontal_list_preview.ml](horizontal_list_preview.ml) and its [interface](horizontal_list_preview.mli). [collections_page.ml](collections_page.ml) mounts it on **Collections**, passing palette and reactive scrollbar description. `C` aliases List_collection and `L` the Bonsai virtual-list adapter.

`extent id` returns 180/204/228/252 pixels. The immutable collection starts with 10,000 integer-keyed cards. `Action.t` separates Grow/Reverse/Prepend/Append. `update` grows an existing extent by 32 up to 500, reorders stable keys, or delegates to `insert`. Insertion chooses a fresh minimum-minus-one or maximum-plus-one ID and stops at 10,100 rows. `B.state_machine0` returns reactive collection state and an action injector; executed actions reduce against the latest source. Constructing an effect does not run it.

A separate reactive axis boolean feeds configuration `let%arr`: estimated 220-pixel width/height, overscan 220, max_active 16, Follow_tail_when_at_end. [`L.component_with_config`](../../lib/bonsai/virtual_list.mli) maps integer keys injectively through Key.of_int into a viewport with height 260 and full assigned width. Horizontal mode also needs its parent-assigned bounded width. Native layout measures extents and asynchronously requests a bounded row set; 10,000 logical entries do not mean 10,000 Bonsai row computations.

`render_row` allocates transient `B.state 0` reactions and derives axis-appropriate dimensions and content. A native reaction click executes the captured `set_clicks (clicks + 1)` assignment, updates that row model and derives its label for native reconciliation. Unlike the collection reducer, this setter captures a rendered next value; do not describe it as a latest-model increment. Repeated effects captured from one render can assign the same value. Surviving rows keep models across axis/configuration changes, but rows retired from the active set follow the managed-row reset contract. The unused lifetime parameter is safe here because rows start no asynchronous work; persistent reactions belong outside transient rows.

The outer `let%arr` reads Output.view/controller/viewport/active_rows. Grow visible derives its target from the observed anchor and is disabled until available. A native Reverse order click injects Reverse, updates keyed order, derives adapter input and triggers native reconciliation around a surviving logical anchor. Controller effects scroll to first, reveal middle or jump to latest; they are guarded by this mounted generation and ignore inactive/absent targets. `V.with_scrollbar` applies the caller’s description.

React to a card, change axes, grow it, and append while at/away from the tail to compare anchor/follow behavior. GPUIO owns viewport/measurement/scrolling; Bonsai owns collection/configuration and transient rows. No fetch worker exists. Adapt with domain-stable keys, external durable row data, handled adapter errors and lifetime guards for asynchronous row callbacks; a mounted-row count is a budget diagnostic, not total data size.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the repository wrapper for the isolated toolchain. These commands were not executed for this documentation change. There is no standalone executable or self-test for this component. Compilation alone does not establish native keyboard, animation, focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
