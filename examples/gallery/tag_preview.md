# Rich tag styling walkthrough

Read [tag_preview.ml](tag_preview.ml) and its [interface](tag_preview.mli). [pages.ml](pages.ml)
mounts it on **Presentation**. `B` aliases `Bonsai.Cont`, `V` `Gpuio_bonsai.View` and `T`
Presentation.Tag. Graph hosts reactive state; `let%arr` derives current tag/control
descriptions.

State machines own variant (Secondary), size (Medium), `Paint.Default` and action count 0. They
return current models plus effect injectors reducing against latest state. Toggles own
outline/rounded/reversed false and label/action true. Native Tag variant activation runs its
injected action, advances variant, and derives updated native paint/readout. Constructing
`act()` for the child button records no action until native activation executes it.

`Paint.t` distinguishes Default/Override/Unset. Default leaves tag hovered opacity 0.9. Override
sets base opacity 0.65 and `Hovered` opacity 0.4. Unset retains base opacity 0.65 while
explicitly removing `Hovered` opacity via `Style.unset`; a base style alone would not remove
helper hover behavior. Native styles handle hover, without an OCaml hover observer. Custom
mode’s initial purple palette is only a state marker here: view derivation replaces it with
current application background/foreground/accent colors.

[`T.create`](../../lib/core/presentation.mli) wraps direct keyed children: selectable Ready · 京都
label and ordinary Tag action. `Reverse` reorders surviving keys, while hiding a slot removes
its native child. Size XS/Small and Medium/Large use two padding/radius groups; rounded
overrides radius 16, width 280 and gap 8 are explicit styles. Outline clears custom background
without changing supplied foreground/border. Root adds no default role/action/focus/live region;
this component explicitly wraps it as named Group.

Native Tag action activation executes its unit-action injector, increments latest counter,
derives readout and updates native text. The tag itself remains content-only, not a
chip-selection/dismissal control. Compare hover paint states, reverse children and change sizes
to exercise ordinary child identity/selection.

GPUIO owns native paint/text selection/button behavior; Bonsai owns options/counter. No
controller/resource/task exists. Adapt with stable child keys and semantic actions supplied by
children. Use typed custom palettes for your application and explicit hover refinements; do not
treat visual semantic color as validation, announcement or a selected business value.

## Run and review

From the repository root:

```sh./scripts/gpuio build examples/gallery/main.exe./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not checks run for this documentation
change. There is no separate executable/self-test for this component. Compilation does not
establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
