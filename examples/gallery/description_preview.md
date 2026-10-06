# Description-list walkthrough

Read [description_preview.ml](description_preview.ml) and its [interface](description_preview.mli). [pages.ml](pages.ml) mounts `component window palette graph` on **Presentation**. `D` aliases [`Presentation.Description_list`](../../lib/core/presentation.mli), while `Editor` is the Eio/Bonsai native text-input adapter.

`B.state_machine0` returns a reactive current model and an action injector producing an effect. Columns start at 3 and cycle 1–10; width indexes cycle four values (720, 607.3, 541, 333.3 pixels); size cycles Medium/Large/XS/Small. Another reducer counts actions. Toggles own axis, borders, spans, separators, reversal, label proportions, extra text, optional action and refinements. Constructing `next_columns ()` or `act ()` does not execute it. `let%arr` reads current values and derives the view when these inputs change.

`Editor.create` supplies a single-line “Description value” field seeded with “Retained draft”. `named` wraps term/definition contents in named accessibility groups. `D.Item.create` builds keyed entries from index-derived `Key.of_int n`; item zero contains the term action and editor, item one an optional Open button, and item two optional wrapping text. Mixed mode changes twelve one-span entries into seven entries with bounded two-column/full-row spans. Reversal reverses entries while keeping their original keys. Two keyed separator bands can be inserted before the fourth packed entry.

`D.create` validates unique keys, columns 1–10 and spans no greater than columns. Packing breaks before an overflowing span; a partial row shares spare width between cells. Horizontal mode uses a 64-pixel or 25-percent label width; Vertical stacks term/definition slots. Slot padding/background refinements and border/size changes alter composition, not editor ownership. Entries remain direct keyed children across packing changes. The outer semantic role is Description_list, named “Workspace details”.

A native click on **Description columns** executes its injected unit action; the reducer receives the latest column model, advances it, and `let%arr` derives valid spans and a newly packed list. GPUIO reconciles the descriptions around the existing keyed editor. Type a draft, change columns/axis, reverse entries, and toggle borders to exercise this continuity. Clicking term/value actions follows the separate counter reducer and derives the action readout. Turning mixed mode on removes entries 7–11; keyed retention cannot preserve descendants of an actually removed entry.

GPUIO owns the editor buffer, focus and layout; Bonsai owns display options/counters. The adapter’s lease retires when its native placement leaves the page; no file/network task or asset registration is started. Adapt with stable domain keys rather than indexes for changing data, validated spans and explicit error handling instead of `ok_exn` for external input. Keep a caller-owned draft if text must survive destruction/remount: layout continuity does not mean persistence across page departure.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks executed for this documentation change. This component has no standalone executable or self-test. Native interaction requires the running gallery; compilation does not establish focus, keyboard, accessibility or platform acceptance. See [gallery instructions](README.md) and the [development environment](../../docs/development.md).
