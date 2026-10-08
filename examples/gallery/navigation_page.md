# Navigation composition walkthrough

Read [navigation_page.ml](navigation_page.ml) and its [interface](navigation_page.mli).
[pages.ml](pages.ml) routes Navigation to `component app window palette graph`. `B` aliases
`Bonsai.Cont`, `V` `Gpuio_bonsai.View` and `Editor` `Gpuio_eio.Text_input`. `graph` hosts
reactive options/controllers; `let%arr` derives current composition from their values, not an
imperative rendering loop.

Pure `choices` validates a Choice collection. `tabs` is Notes/Draft; `breadcrumb_path` is
Studio/Workspace/Preview; `variant_name` formats the five Tab_bar variants. State initially
selects Notes and Preview route, enables decorated labels, disables custom styles/passive
Workspace, and uses Underline variant. A state machine cycles variants, returning current model
plus action-effect injector reduced against latest state. Creating setters/injected effects does
not run them.

Native Draft tab selection calls `set_tab`, changes the reactive selected ID and causes
`let%arr` to derive Choice.Config plus active flags for two keyed `V.tab_panel` wrappers. GPUIO
updates tabs and makes the Notes panel inactive while retaining its native editor. Editors are
created once outside selection, seeded separately for Notes and Draft. This is retained native
content, not `match%sub` destroying the unselected computation. Decorations are passive
badge/label views; appearance styles do not own selection or focus.

Breadcrumbs derive the current prefix of the fixed path using route identity. Native Studio
activation executes `set_route`, updates the route model, derives a shorter validated breadcrumb
collection and updates native content/readout. Open Preview restores the full prefix. Passive
Workspace disables navigation only for that breadcrumb item; it does not prevent the explicit
Open Preview setter. This is sample route state, not filesystem navigation.

The first card embeds the tab content and an Inspector in `V.split_pane`, with bounded
palette-scaled height 220 and first-size seed 400/minima 200 and 110. Native pointer/keyboard
resizing owns measured sizes; initial sizes are not reset by each view derivation. Separate
child guides explain [closable tab parts](tab_content_preview.md),
[flat split panels](split_group_preview.md), [workflow stepper](stepper_preview.md),
[disclosure](disclosure_preview.md) and [pagination](pagination_preview.md). They allocate
independent models/resources; the page composes their outputs rather than copying those models
into one navigation state.

Type into both panels, switch tabs/variants, resize and navigate breadcrumbs to compare state
ownership. GPUIO owns native editors/focus/dividers; Bonsai owns selections/options and adapters
own leases. Child asset scopes retire on page departure; no page-level file/network task is
started. Adapt with stable domain IDs, typed controlled selections and explicit task/draft
lifetime. Do not infer storage or route loading from retained native panels or breadcrumb
labels.

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

For a focused tab walkthrough, run
`python3 scripts/test_gallery.py --section tabs --images scratch/gallery-tabs`.
The driver types into Notes, changes label presentation and variants, and checks
that the retained editor keeps its draft. It then exercises overflow and the
independent close/menu controls described in [tab content](tab_content_preview.md).
Thirty cases combine five variants with both themes and three preview scales:
the configured 160/140-pixel target widths and 40-pixel height remain explicit,
while Right/Left selects Draft/Notes through the native tab group and Bonsai state.

These are two different lifetimes: `V.tab_panel ~active:false` preserves the editor
while switching tabs, but leaving the Navigation page retires its native editors.
Returning recreates Notes from its initial text. Applications that want drafts
to survive page retirement must keep that data in a longer-lived model. This
walkthrough checks the example's actual lifetime; it is not persistence logic.
It does not operate VoiceOver or measure the indicator's presentation timing.
