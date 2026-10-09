# Standalone radio navigation walkthrough

Read [checkable_navigation_preview.ml](checkable_navigation_preview.ml) and its [interface](checkable_navigation_preview.mli). [pages.ml](pages.ml) mounts it on **Selection & actions**, in “Compose a choice your way”.

`B.state "balanced"` owns application selection. Four `B.toggle` values own custom order, skipping Fast, disabling controls and detailed labels; only detailed labels starts true. `let%arr` reads the values to build three standalone controls from `(id, title, detail)` tuples. `String.equal selected id` derives each checked state; `set_selected id` is its selection effect. Selecting an already selected item assigns the same value rather than clearing it.

The selected value is reactive, and `set_selected id` constructs a setter effect without executing it. Native activation of Deep executes that setter, replaces the selected model with `"deep"`, and causes `let%arr` to derive checked=false for Balanced and checked=true for Deep. GPUIO reconciles those descriptions and the readout on the existing keyed controls. The native event does not give each radio an independent selection model; the single Bonsai value remains authoritative.

Each radio receives key `standalone-<id>` and [`Radio.Position.create`](../../lib/core/radio.mli) with zero-based index and count 3. That validated value informs accessibility position; it does not implement group navigation. The rich branch uses `V.radio_with_label`, a passive two-line column and explicit accessible name. The simple branch uses `V.radio`. An outer `Radio_group Vertical` semantic role names the set “Response depth”; application code still owns membership and selection.

An optional [`Tab_order`](../../lib/core/tab_order.mli) sets `tab_stop=false` only for Fast when requested. Custom ordering supplies indices 0, -1, -2, reversing their sequential order without reordering the source list or position metadata. Negative indices are ordering values, not automatic Tab exclusion. These standalone controls have individual Tab stops; use the managed radio-group API for arrow-key navigation rather than assuming a semantic wrapper creates it.

Select Deep, enable Skip Fast with Tab, and traverse: Fast remains pointer-selectable. Enable Reverse choice Tab order to change traversal while retaining selected Deep. Disable standalone choices: the application still retains its selection. Turning both navigation options off restores defaults on the same keyed controls. The displayed depth is application state, not an independent native model.

No Eio task, borrowed asset or controller is allocated here. Native GPUIO owns focus, activation and accessibility; Bonsai owns the controlled values and effects, retired with this page. Adapt by using a typed choice type instead of strings for dynamic application data, maintaining valid membership/position values, and distinguishing independent radios from a managed navigation group.

## Run and review

From the repository root, use the isolated repository wrapper:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These are instructions, not validation performed for this documentation change. This component has no standalone executable or self-test. Interactive behavior requires the native gallery; compilation alone does not establish keyboard, focus, accessibility or platform acceptance. See the [gallery README](README.md) and [development environment](../../docs/development.md).
