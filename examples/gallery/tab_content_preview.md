# Independent tab parts and reveal walkthrough

Read [tab_content_preview.ml](tab_content_preview.ml) and its
[interface](tab_content_preview.mli). [navigation_page.ml](navigation_page.ml) mounts it on
**Navigation**. `B` aliases `Bonsai.Cont` and `V` `Gpuio_bonsai.View`; graph hosts reactive
state/scope computations, and `let%arr` reads current values to derive views.

Local `Model.t` owns open tab names, optional selected name, reveal serial/request and last
action. Initially six tabs are open and Plan selected. Actions are `Select`, `Close`, `Reveal`,
`Reverse` and `Restore`. `B.state_machine0` returns current model plus injector; executed
effects reduce against latest state. `Close` removes a name and selects the first remaining tab
only if closing the selected one. `Reverse` keeps selection by name; `Restore` resets contents
while preserving serial. `Reveal` increments a positive serial with saturation check and does
not select anything.

Native `Close` review activation executes its `Close` effect, changes `open_tabs`/current
fallback, and makes `let%arr` derive new Choice collection/content/menu icons. GPUIO removes
that owner while preserving surviving keyed tabs. Tab selection instead injects `Select` and
derives controlled selected metadata. Constructing those effects does nothing immediately. The
reducer trusts `Select` and `Reveal` names from known controls/current native choices; it is not
a general validation boundary for arbitrary external names.

`V.tab_bar_with_content` uses key closable-tabs, Pill appearance, optional native motion and
bounded horizontal viewport. Each `Tab_content` has decorative prefix and independent
close-button suffix. Part hit areas do not select the tab; suffix actions keep their own
focus/owners. Narrow bounds labels to 190 pixels; configured full names remain accessible/menu
labels. [`Tab_bar.Viewport`](../../lib/core/tab_bar.mli) retains native offsets. `Select` last
changes selection without scrolling; `Reveal` last submits one-shot least-distance reveal
without changing focus/selection. Requests are not acknowledgements and old serials do not
replay.

[Preview_scope](preview_scope.md) registers a decorative SVG for menu rows. Loading/failure
omits menu icons but leaves tabs usable; showing icons borrows the scoped registration. Native
menu readers retain their leases even while closed, retiring when removed. `Tab_bar.frame` adds
outside-viewport Workspace prefix/restore buttons/all-tabs menu within width 560/`Max_width`
100%. Menu selection uses the same selection callback without implicit reveal.

Close, reverse and restore tabs, then compare `Select` last and `Reveal` last. Motion is native
and separate from selection/focus; no editor/page-panel content or background loader exists.
Bonsai owns model/options, GPUIO owns focus/scroll/menu/motion, scope owns asset registration.
Adapt with stable IDs, validated external actions, monotonic reveal serials and separate content
lifetimes; this example does not open workspace documents.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this
documentation change. There is no standalone executable or self-test for this component.
Compilation does not establish native focus, keyboard, animation or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
