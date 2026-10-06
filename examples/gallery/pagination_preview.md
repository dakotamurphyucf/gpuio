# Controlled pagination walkthrough

[pagination_preview.ml](pagination_preview.ml) has no separate interface. [navigation_page.ml](navigation_page.ml) mounts it on **Navigation**, in “Navigation at any scale”. Local `action` is Request/Count/Toggle_disabled.

`B.state_machine0` owns a reactive [`Pagination.t`](../../lib/core/pagination.mli), initially page 60 of 120. It returns current model plus an action-injection effect constructor. Its reducer applies typed requests to the latest model, validates new totals or toggles disabled. `B.toggle` owns compact appearance; `B.map` derives Full/Compact layout. Constructing `inject (Count 3)` does not run it. Native **Shrink to 3** activation executes that effect, clamps current page to 3 in the model, and causes `let%arr` and the controller to derive updated navigation/readout for native reconciliation.

Pages are one-based, total 0–1,000,000,000. Empty has no current page; growth from zero chooses 1. Disabled suppresses user requests but not programmatic total changes. `Pagination.items` emits at most 13 endpoint/neighborhood/gap items; it never allocates a billion page buttons. Requests at boundaries, disabled requests and stale absolute targets are no-ops.

[`Gpuio_eio.Pagination.create`](../../lib/eio/pagination.mli) binds the model/layout and reactive request callback. `Controller.view` mounts its native gap chooser with a named 420-pixel overlay and outside-pointer dismissal. A native Next event injects Request Next, the reducer advances the latest current page, and reactive derivation updates the native view. The controller handles transient opening/confirmation and one native numeric editor; each opening has a fresh field identity, with at most seven gap shortcuts.

Open an ellipsis, type a page and use **Go to page**. Enter normalizes the numeric draft but does not navigate. Confirmation reads the actual native draft, clamps/rounds within the gap and revalidates opening/current model before requesting selection. Composition/rejection keeps the popup and exposes errors; concurrent confirmations coalesce. Model changes/disabling/deactivation discard the opening, and delayed replies cannot navigate/dismiss a newer opening. Compact closes the chooser and shows previous/next controls. Closed anchors retain identity and closing restores the owning gap button’s focus.

There is no page-data loading or network task: navigation changes only the demo model. GPUIO owns field/focus/overlay interaction, the Eio adapter owns transient request fencing, and Bonsai owns pagination state. Adapt by applying requests against current data and starting your own scoped loads after selection. Handle failures separately; the pagination model is not a data-fetch scheduler.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the repository wrapper for the isolated toolchain. These commands were not executed for this documentation change. There is no standalone executable or self-test for this component. Compilation alone does not establish native keyboard, animation, focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
