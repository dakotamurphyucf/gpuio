# Pie inspection: keyed rows, Bonsai actions and one native editor

[chart_content.ml](chart_content.ml) adds ordinary GPUIO child Views to pie
inspection. Its [interface](chart_content.mli) exposes a reactive `component`,
stateless `controls`, and `content` derived from an optional dataset. The
[Charts page](charts_page.md) owns the source registration and calls these three
functions; this module never registers or publishes chart data.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Charts & data → Pie**, then **Inspection rows**, **Interactive
inspection** or **Inspection overlay**. Focus the pie, press Home to inspect the
first slice, then Tab to enter its details. Interactive modes add **Inspect
reasoning** and **Inspection note** to that slice. The [local root and installed-consumer walkthrough](../../docs/evidence/chart-inspection-gallery-och41.md)
qualifies these interactions and records the precise limits.
No external files, network services or credentials are required.

## Reactive controller and effects

The aliases are `B = Bonsai.Cont`, `V = Gpuio_bonsai.View`,
`Rows = Presentation.Chart_inspection` and `Content = Chart_inspection_content`.
`Mode.t` is a closed variant: Native, Rows, Interactive or Overlay. Its derived
`equal` supports typed preset comparison, and `all`/`label` generate the controls.
The abstract `t` contains the current palette/mode/count, their effects and the
editor controller; callers use its functions rather than modifying the record.

`component window palette graph` allocates two `B.state` cells: Native mode and
zero activations. The Bonsai `graph` owns these retained application models.
Each state call returns a reactive current value and a setter; invoking a setter
constructs an effect that changes the model when an event executes it.
`Gpuio_eio.Text_input.create` allocates one editor controller for this window,
using a constant reactive config from `B.return`: a single-line field with label
Inspection note and placeholder Add context. It is created once in the graph,
not once per slice or on every render.

The `let%arr` reads the palette, model values, setters and controller together,
then derives `t`. When one input changes Bonsai recomputes this record. This is
view derivation, not an event loop. `activate = set_activations (activations + 1)`
is the current increment effect; the next derivation supplies a fresh effect
using the updated count. `controls t` maps the four typed modes into
`Palette.button` calls carrying `t.set_mode candidate`, and shows the count.
Neither constructing a button nor deriving the record executes those effects.

Trace: choose Interactive inspection → native preset button runs its setter
→ Bonsai changes mode → the page's outer `let%arr` reads the new controller value
→ `content` attaches interactive Views → activate Inspect reasoning → the native
button runs `t.activate` → Bonsai increments the count → both the button caption
and the controls' count text derive again. This interaction publishes no data.

## Source values and typed attachments

The caller passes `Registered.data source.chart` into `content`. The function
matches mode and `Chart_data.Expert.contents`: Native or absent data yields
`Content.empty`; non-pie families also yield empty content. Pie data in another
mode produces one entry per slice. The page displays the controls and attaches
the content only in its Pie family, so other demonstrations keep native summaries.

`List.sum (module Float)` computes the total from source slice values.
`slice_content` formats weight with `%.0f` and share with `%.1f%%`; a nonpositive
total yields zero share. These are application formatting choices, not chart
geometry or native number formatting. `Rows.view (Palette.appearance p)` receives
an ordinary title View and two `Row.text` values with stable keys `weight` and
`share`. Each row supplies color, label and formatted value. Swatches are
decorative; the labels provide meaning without color. The helper supplies keyed
layout wrappers and semantic description-list/term/definition roles.

Each `Content.Entry.create` uses `Content.Target.slice (Chart_data.Slice.id slice)`.
This typed ID matches source identity rather than label or list position.
`Content.create` validates the entry collection; `Rows.view` separately rejects
duplicate row keys. `ok = Or_error.ok_exn` unwraps these known-valid sample
constructions. When adapting to user-controlled data, propagate constructor
errors and render a useful notice instead of assuming that invariant.
The page attaches the result using `V.chart ~inspection_content` on its existing
`gallery-chart` key. Matching inspection replaces the native summary; unmatched
targets retain the summary.

The source getter and native chart preparation are asynchronous boundaries.
`Registered.data` returns the latest desired dataset, which may lead native
acceptance and the displayed chart. Rows describe that desired snapshot; this module neither checks
`Registered.is_published` nor synchronously waits for native acceptance/painting.
After Update chart samples, do not interpret a temporarily displayed weight or
share as proof that the newest publication is on screen. The page's source and
Ready notices own that update path. Singular slice targets follow IDs across
publications; aggregate content in another application must instead use
`Target.of_selection` and its exact resource/generation/revision contract.

## One editor placement and stable layout

Rows mode attaches only formatted rows. Interactive uses Card containers;
Overlay uses plot-sized Overlay containers. In both interactive modes only
stable slice ID 1 receives the action and `Gpuio_eio.Text_input.view t.editor`.
Other slices receive rows alone. The explicit typed ID check is essential:
a controller represents one native editor placement, and placing its View in
multiple slice entries would violate that ownership contract.

The placement/content wrappers keep the same keys `inspection-placement` and
`inspection-content` in Card and Overlay. The rows, action and note keep keys
`rows`, `action` and `note`. Switching Interactive ↔ Overlay changes the entry's
container metadata and positioning context without changing this child structure.
The outer wrapper aligns at the end; the inner panel has a palette-scaled
240-logical-pixel width capped by its parent, padding, surface background and
border. The editor's height/font also use `Palette.size`. Theme derivation updates
ordinary styles and colors; it does not issue a text replacement command.

Rust owns the native draft, selection, IME session and undo history.
The controller observes that session; this sample supplies no application draft
mirror or initial-text restoration. Switching to Rows or Native removes the
interactive editor View and destroys its native draft. Returning mounts a fresh
editor. Removing pie content by choosing a non-pie family has the same placement
consequence. The activation count belongs to Bonsai, so these content changes do
not reset it while the page graph remains retained. Hiding an inspection because
another target is previewed is distinct from removing the editor entry.

`Preview_scope` owns the chart registration's page-visit lifetime; retaining an
entry or editor controller cannot keep cancelled chart data alive. No polling,
file/network I/O or asynchronous task is started by this module. Review
[Text_input ownership](../../lib/eio/text_input.mli),
[content targets](../../lib/core/chart_inspection_content.mli) and the
[experimental attachment contract](../../lib/core/view.mli) before adapting it.

Manual review should compare Card/Overlay, action counts, drafts across retained
layout/theme changes and deliberate removal, source updates, keyboard entry and
page teardown. The linked walkthrough records actual local native behavior. It does not establish
physical IME candidate-panel, VoiceOver, Linux desktop or release acceptance.
