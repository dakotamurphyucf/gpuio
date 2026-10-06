# Button appearance walkthrough

Read [button_appearance_preview.ml](button_appearance_preview.ml) and its [interface](button_appearance_preview.mli). [pages.ml](pages.ml) mounts `component palette graph` on **Selection & actions**, in “Actions, in context”.

`Variant.t` enumerates eleven demo styles; `all`, `label` and `accent` provide ordering, names and theme-aware colors. These are application styling choices, not a GPUIO semantic variant enum. The pure `appearance` function combines variant and outline/compact/large/rounded/selected flags into validated `Style.t`. Filled variants use accent backgrounds; selected appearance overrides that choice. Text/Link omit fixed height and horizontal padding. Custom adds a shadow. Focused and Disabled styles are always supplied; Hovered/Pressed styles are omitted when selected. GPUIO applies these native state styles without an OCaml repaint loop.

Seven `B.toggle` calls own appearance/loading/disabled choices. Two `B.state_machine0` reducers own the optional hovered variant and `(request_count, last_variant)`. A hover-enter stores the variant; a leave clears it only if it is still current, preventing a late leave from clearing another hovered action. `let%arr` reads state and derives all buttons. `invoke variant` is an effect executed on activation, not while mapping `Variant.all`.

These Bonsai models are reactive inputs to view derivation. `state_machine0` returns its current model and an action injector; when the injected effect runs, the reducer receives the latest model. For example, native activation of Primary executes `invoke Primary`, the request reducer increments the latest count and records Primary, and `let%arr` derives the new request readout for native reconciliation. Merely constructing that effect while building a button records no request. Native hover similarly injects `(variant, hovered)` into its own reducer; it does not drive the native state styles.

Each action has a stable `appearance-<label>` key. Most use `V.button`; Link uses [`Link.Config.create`](../../lib/core/link.mli) and `V.link`, with a meaningful accessible name and application-owned click effect. **Load link preview** affects only Link: it changes its visible text, exposes busy state and suppresses activation while retaining normal focus eligibility. It starts no asynchronous job or navigation. Disabled takes precedence and affects every action.

`V.with_hover` reports native hover transitions for the readout. `V.tooltip` wraps each action, with a rich two-line Primary tip placed right and simple tips above other actions, all at offset 8. [`Tooltip.Config`](../../lib/core/tooltip.mli) defaults to native-managed transient visibility: focus opens immediately and hover follows native delay policy. This file stores no tooltip-open Boolean.

Activate Primary to increment requests; select rounded/outline and observe the same actions with new styling. Enable selected appearance: it changes emphasis but does not create toggle semantics. Enable link loading: Link cannot record a request, while other actions still can. Recover and activate Link to record a mock request. Hover reports transitions; it is not the source of hover styling.

There are no assets, Eio workers or explicit disposal callbacks. Bonsai owns preferences/counters; native owners manage focus, hover and tooltips and retire on page removal. Adapt `appearance` for your design system and replace request counting with actual effects. Keep selected semantic state explicit if these become toggle controls, and compose busy artwork rather than assuming a loading flag draws it.

## Run and review

From the repository root, use the isolated repository wrapper:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These are instructions, not validation performed for this documentation change. This component has no standalone executable or self-test. Interactive behavior requires the native gallery; compilation alone does not establish keyboard, focus, accessibility or platform acceptance. See the [gallery README](README.md) and [development environment](../../docs/development.md).
