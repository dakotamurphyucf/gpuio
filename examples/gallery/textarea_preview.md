# Multiline layout and geometry walkthrough

Read [textarea_preview.ml](textarea_preview.ml) and its [interface](textarea_preview.mli). [pages.ml](pages.ml) mounts it in **Text editing → Multiline**. Reactive wrap/indent start true; whitespace/cursor-margin start false, with an initial notice. Configuration `let%arr` builds [`Text_area_layout`](../../lib/core/text_area_layout.mli) and searchable multiline Text_input.Config with eight rows and submit_on_enter false.

`B` aliases `Bonsai.Cont`, `V` aliases `Gpuio_bonsai.View`, `Editor` aliases `Gpuio_eio.Text_input`, and `Scope` aliases `Gpuio_eio.Scope`. `graph` hosts Bonsai state/controller computations. Palette and layout options are reactive values: `let%arr` reads their current contents to derive configuration or view descriptions as inputs change. Scope controls application work lifetime; it is separate from that reactive graph.

Editor.create seeds long indented notes once; a separate Search_bar controller targets that same editor. Native Wrap long lines activation executes the toggle effect, updates the model and derives layout/configuration; GPUIO reflows the existing editor without replacing its draft, selection or history. Constructing effects does not execute them. Wrapping resets horizontal scrolling; offsets can clamp. Indent chooses Match_first_line/Flush_left; explicit cursor margin 3 rendered lines is bounded by native viewport policy.

`scroll` sequences an asynchronous native scroll_to, then sets notice from its result; scrolling does not move selection. `inspect` reads the latest completed viewport and reports buffer-line range/offset, or not-yet-laid-out. These effect `let%bind` operations wait for replies, unlike reactive `let%arr` view derivation. Search_bar.wrap presents native search/replace controls; open_ effects request Find or Find and replace without replacing the editor controller.

Selection inspection has stronger fencing. A visit-owned [Preview_scope](preview_scope.md) installs cancellation that clears mutable reactive `range_busy`. `begin_range` accepts only one active request. It reads a fresh native snapshot, checks scope activity, then requests range_bounds for that exact snapshot selection. `finish_range` clears busy and permits publication only while the scope is active. Stale_revision asks the user to inspect again; None means the selection is outside completed layout. The eventual notice setter updates its model, makes outer `let%arr` derive readout and button availability, and reconciles native text. It reports geometry without reading the clipboard.

Type/select notes, change layout, jump to end, then inspect viewport and selected range. Auxiliary buttons use Button.Focus.Preserve; range inspection disables while busy/unavailable. During composition the visible notice is “Composing text…”. Native GPUIO owns text/search/layout/geometry; adapters own exact editor requests; Bonsai owns options/notice and busy state. Scope cancellation suppresses departed range publication, but ordinary scroll/viewport notices use their command results directly, without that range-specific guard.

Adapt with handled geometry errors, exact snapshots and your desired task lifetime. Do not treat stale last observations as current layout or preserve destroyed draft/undo state implicitly. This demo starts no file/network worker or asset registration.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not executed checks for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native keyboard/IME/focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
