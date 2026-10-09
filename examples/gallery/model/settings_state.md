# Settings state: retained values, catalog metadata and committed export

[settings_state.ml](settings_state.ml) and its [interface](settings_state.mli) own
the gallery's application settings data outside transient native rows. The pure
model validates actions, rebuilds presentation metadata and encodes an example
export. It creates no Bonsai graph, editor, file dialog, Eio writer or resource scope.
[settings_preview.ml](../settings_preview.ml) supplies those independent layers.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Settings**. Edit name/budget, switch Workspace/Advanced, filter preferences,
change a feature and reset dirty fields. Export settings opens the caller's native
Save panel and writes a preview S-expression; normal edits remain local without
export. There are no model-specific flags or source assets. See
[development](../../../docs/development.md) for toolchain setup and
[gallery README](../README.md) for existing evidence/platform limits. No build,
file-save, IME or GUI test was run for this documentation review.

Read Field, defaults, dirty/disabled/pages, accessors, Action/reset helpers, apply
and encode. Public model t is abstract with typed equality; internally it stores
immutable values and a `Gpuio.Settings.t` catalog. Values contain name, notifications,
reports, committed budget **and a separate raw budget draft**, region/model indices,
custom count, a typed Int.Set of feature numbers and lock policy.

Defaults are Northstar, notifications=true, reports=false, budget 25 with draft
“25”, region/model/custom=0, no features and locked=true. `Field.t` names eight
Workspace fields plus Feature n for the 48 Advanced items. Stable IDs such as
workspace-name and feature-47 are not row positions or native node IDs.
`Field.of_id` searches the fixed 56-field domain with typed Settings.Item_id equality;
a directly constructed Feature outside 0–47 is not a catalog field.

## Catalog metadata follows values without owning the controls

`dirty` compares each value with defaults; Budget compares both committed value and
raw draft. Locked is never dirty. `disabled` makes the Organization policy row
always unavailable and Custom unavailable while the lock Boolean is true. Unlocking
Custom does not turn the separate Organization policy row into an editable control.
`pages` builds Workspace groups Identity/Generation/Managed preferences and 48
Advanced experiment groups. `S.Item.create` supplies stable ID, title/help,
horizontal/vertical layout, disabled state and Dirty/Clean reset metadata; Custom
uses `S.Item.custom` with search keywords instead of ordinary title/description.
Workspace is default-open. The model stores no control View inside this catalog.

`initial` validates that catalog with `S.create`. `Navigate request` changes only
catalog search/selection/expansion through `S.apply_request`; application values
remain intact. Other successful actions rebuild metadata with `S.with_pages`,
preserving surviving navigation preferences and expansion by ID. If values are
unchanged, apply returns the same model. The
[settings interface](../../../lib/core/settings.mli) defines metadata bounds and
stable preference behavior; it does not create native widgets or execute resets.

`reset_targets t scope` asks the latest catalog for eligible IDs then resolves them
back to Field variants. Matching-page/group scopes honor the current filter/navigation;
whole-page scopes can include filtered-out items. Disabled/clean/nonmatching fields
are skipped. `can_reset` checks the field's current item scope, preventing a delayed
reset from bypassing newer lock or dirty metadata.

## Validated actions and native editor boundaries

`apply` returns `t Or_error.t`. Name uses `Text_input.validate_text ~mode:Single_line`;
a blank name is allowed during editing but rejected at export. Budget requires a
nonempty finite committed Number in 0–1000, preserving its independently typed draft
without demanding that every partially typed spelling parse as the committed number.
Region accepts 0–2; Model 0–249; Toggle_feature 0–47; invalid indices return errors.
Custom increments only while unlocked; Toggle_lock changes that policy. Reset restores
an eligible field's default, including both budget representations; Locked reset
leaves values unchanged. `Or_error.Let_syntax`'s `let%bind`/`let%map` here propagate
validation failures synchronously; they are not Bonsai reactive syntax.

In the caller, a window/page graph creates a `B.Expert.Var` for Model.initial.
Its action effect invokes `Model.apply` against the Var's **latest** value, setting
it only when typed equality detects a change. `B.map` derives catalog for
`Gpuio_bonsai.Settings.component`; the caller's `let%arr` derives field Views from
current values. Managed native rows can be evicted while these application values
survive. Name and number controllers separately own selection/history/composition;
[Editor_visit](editor_visit.md) plus managed-row lifetimes fence persistence callbacks.

For a concrete budget edit, commit 37 and leave the raw draft “1e-” before navigating
away. Noncomposing accepted observations update Budget (37,“1e-”); the model marks
that field dirty and retains both values across navigation. Export later writes
37, not “1e-”. Reset while the row is mounted first reads the controller snapshot,
rechecks current reset targets/visit, issues revision-and-lease-guarded native
replacement, and mirrors its successful observation. The pure Reset action sends
no editor command; using it alone for a still-mounted editor could make application
metadata disagree with native text. An unplaced field can reset application values
directly, with a future mount seeded from them.

## Export is a snapshot, not file I/O

`encode` rejects blank/whitespace-only workspace name, then produces a newline-terminated
S-expression headed by gpuio-settings-preview-v1. It contains name, notifications,
reports, committed budget, region/model/custom and sorted feature numbers. It omits
budget draft, native selection/history, lock policy, search/navigation and caller
busy/error state. This is a demo export format, not a GPUIO persistence contract or
an import/decoder API.

The caller takes the encoded snapshot before opening its Save panel; later edits
remain independent. It supplies the actual Eio save callback and visit scope, busy
handling, cancellation/stale-result fencing and failed-export demonstration. The
[file adapter](../files/settings_file.ml) and its
[interface](../files/settings_file.mli) enforce the 64 KiB export limit without
truncating or discarding larger retained editor drafts. Encoding alone does not
establish a file was written or a native frame painted.

The [existing expect tests](../../../test/gallery/settings_state_test.ml) cover
retained drafts/navigation, latest queued actions, disabled Custom/reset, filtered
reset scopes, committed-only export, invalid fields and the 56 stable IDs. No test
source group or fresh execution is claimed here. Custom currently uses plain int
increment without a saturation guard; an extremely long-lived adaptation should
bound that demo counter rather than permitting integer wraparound. This finite
preview has no setter that injects an arbitrary custom count.

To add a field, extend Field/all/ID mapping, values/defaults, dirty/disabled metadata,
page catalog, typed action validation/reset and the caller's exhaustive row renderer.
Keep export inclusion deliberate and version its format if changing it. Preserve
separate draft/committed ownership, stable IDs and guarded native reset completion;
application persistence must not depend on an evictable row's lifetime.
