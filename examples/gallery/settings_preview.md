# Settings rows, native editor visits and snapshot export

[settings_preview.ml](settings_preview.ml) and [settings_preview.mli](settings_preview.mli)
build the Settings gallery composite. Read constants/choices, retained data/controllers,
observation mirroring, guarded reset, export scope/effect, presentation state,
Panel.component/render_item and final controls. `B = Bonsai.Cont` constructs reactive
computations; `E = Bonsai.Effect` sequences later actions; `V` describes native views.
`Panel = Gpuio_bonsai.Settings` is the public composite, while `Model` is the pure
[Settings_state](model/settings_state.md), not framework-managed persistence.

From the root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section settings
python3 scripts/test_gallery.py --section settings-composition
```

Choose Settings. The fixture has Northstar defaults, three regions, 250 deterministic
Model labels and 48 Advanced feature rows. No server/backend dataset is queried.
The physical macOS harness covers native input/panels separately from pure model/
file tests; [README](README.md) records current acceptance and Linux limits.
Commands here are reviewed, not executed as documentation evidence.

## Retained data and reactive syntax

component creates data/status/busy/query_error Vars outside render_item. update
runs Model.apply against latest data, updates only on typed Model.equal change and
reports errors; action wraps that mutation in E.of_thunk. Text.create makes name
and search controllers; Number.create makes budget controller with finite numeric
domain 0–1000/step 1 and allow_empty=false. B.return supplies static validated configs.
These controllers belong to the preview/window graph, not an evictable row.

B.Expert.Var.value observes application data; B.map derives catalog/appearance;
let%arr combines changing values into views/callbacks. B.Edge.on_change observes
search's native snapshot and validates Settings.Query: valid input navigates Search,
invalid input reports query_error without replacing current query. It does not
perform file/network search. B.peek permits a later reset effect to obtain Active
or Inactive controller computation. E.Let_syntax let%bind/let%map sequence effects,
whereas B.Let_syntax let%arr derives reactive values without awaiting I/O.

## Persist observations only from an active row visit

Each transient renderer creates [Editor_visit](model/editor_visit.md) via Expert.thunk.
Lifecycle activation records the name/budget placement in a shared optional visit;
deactivation permanently retires it and clears the pointer only if still identical.
Managed_rows.Lifetime.guard checks callback execution lifetime. This complements
native editor lease/revision validation; retained controllers can remember an old
snapshot after their native row disappears.

mirror_name extracts revision/text/composition from Changed or Submitted, ignores
Search_changed and records only active, increasing nonnegative revisions through
Visit.observe. Composing text is provisional and does not update Model.Name.
mirror_number extracts the snapshot from every numeric event variant (including
Step_requested), checks visit/revision and no composition, then stores committed
value **and** typed draft separately. A draft such as 1e- can remain unfinished while
the committed value is 25. Native text, selection/IME/undo belong to Rust; observations
never implicitly replace that text. The public [Text](../../lib/eio/text_input.mli)
and [Number](../../lib/eio/number_input.mli) adapters define those boundaries.

On remount, Text.view uses Model.name and Number.view uses Model.budget/budget_draft
as mount-only seeds. They restore accepted application values/draft, not native
selection/history or provisional composition. An unchanged seed during rendering
does not overwrite a live editor. These controllers are placed once, despite
native horizontal/vertical layout changes.

## Reset against current metadata and exact native state

reset resolves Model.reset_targets from latest data when handled, sequentially
folding fields and reporting aggregated errors. reset_field rechecks that field
is still allowed under its Reset_scope. For an unplaced name/budget, pure Reset is
safe and future mount seeds reflect it. For an active visit it peeks the controller,
reads an exact native snapshot, rechecks current scope policy plus visit activity,
then performs replace_if_unchanged or replace_value_if_unchanged with selection End
and undo Record. The expected lease/revision guards prevent a newer edit/remount
receiving the replacement; active composition is rejected natively. Errors preserve
native/application data and are reported.

Successful command replies are explicitly mirrored through the same visit check;
ordinary on_event callbacks do not run for command replies. Other fields use pure
Reset. A row Reset button also wraps its effect with Lifetime.guard, while model
can_reset controls its current disabled state. The model checks lock/dirty/filter
policy again, so a queued action cannot bypass changed metadata. Reset is not
implemented by rerendering initial_text or resetting only the application Var.

Concrete trace: type budget 37, commit, then draft 1e- → accepted noncomposing native
observation stores(37,1e-) → scroll/navigation evicts row → retained data survives →
new row mount restores both → Reset reads exact native stamp and default value 25 →
successful reply mirrors(25,25), clearing dirty metadata. IME intermediate text
instead stays native until a composition-free observation is admitted.

## Composite catalog and exhaustive row renderer

Panel.component receives appearance, Model.catalog, group variant/size, bounded
height 510 and either width 610 or full. Search supplies one native Text.view;
on_request reduces Navigate, on_reset receives scopes rather than captured setters.
Page suffix views display 8 for Workspace and 48 for Advanced. Invalid composite
configuration returns Error and the final view shows its message.
The [public composite contract](../../lib/bonsai/settings_panel.mli) specifies
transient managed groups/items, stable IDs, disabled custom content and native
layout below 480 logical content pixels. Parent sizing is explicit; panel resizes
styles on one control placement rather than duplicating editor controllers.

render_item resolves typed Field from Settings.Item.id and derives current layout/
size/data/controllers/palette. Name uses Settings_field.control with help/composition
and blank-name error; Budget parses draft into Empty/Incomplete/Invalid/Out_of_range/
Valid messages, retaining text until Enter commit/Escape cancel. Notifications is
a switch, Reports a checkbox, Region and Model typed Choices.select (model menu
max seven visible rows), Custom an ordinary button guarded by lifetime, Locked an
always-disabled organization switch, and Feature n a controlled switch. Semantic
field metadata carries label/help/error; each native control carries its own
accessible name. Native disabled wrappers block even custom button input and
Model.apply rechecks policy. No actual OS notifications are requested by the
Notifications preference switch in this preview.

Width toggles full/610; variant cycles Outline→Filled→Plain→Card; size cycles
Medium→Large→Small. These small state_machine0 computations derive styles, not
storage operations. Final labels expose retained values and error/status output.

## Export captures committed values and owns work by visit scope

Preview_scope.acquire creates settings-export as a child of the exact window scope
on activation, cancels it on departure and acquires anew on return. Scope.on_cancel
clears busy. Loading/Failed means export buttons' effects Ignore; Ready supplies
the active scope. Field values/controllers stay outside transient rows, and export
work is independent of any particular row but cancelled with this preview visit.

save_effect checks current scope/busy and Model.encode before opening a Save panel.
Blank name fails without a panel. The versioned payload captures committed values
before panel selection; numeric draft/search/navigation/native history/busy state
are excluded. Later edits remain independent. Busy spans panel and write. The panel
starts at /tmp with gpuio-settings.sexp; cancellation clears busy without claiming
success. After its reply the code rechecks scope activity. Scope.start passes
selected File_path and immutable bytes to injected save. Its on_result flattens
task/writer Or_error, clears busy and reports success/failure; cancelled/retired
scope suppresses late delivery. Scope admission failures are reported immediately.

Try failed export sets fail=true, skips the panel via Ok None and starts a scoped
fake writer failure. It demonstrates error reporting while keeping edits; it is
not a real filesystem failure. [Application](application.ml) injects Eio fs and
secure_random into [Settings_file](files/settings_file.md), which enforces 64 KiB and
atomic private replacement. A cancellation after rename can still leave an export
on disk even though its UI result is suppressed. Export is a demo format with no
import path here, not framework settings persistence or a saved/dirty baseline.

To add a field, extend model ID/default/validation/reset/catalog/export policy and
this exhaustive renderer together. Keep application drafts outside evictable rows,
use mount-only seeds, and guard both asynchronous reset completion and export
scope. For stronger persistence policy, define a separate application owner rather
than letting row unmount decide what data is saved. Native harnesses and pure file/
model tests cover different boundaries; source reading does not prove GUI/IME acceptance.
