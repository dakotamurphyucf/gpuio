# Managed table behavior

`Table.Config` exposes row-header visibility, `Table.Boundary.Stop | Wrap`, and
an optional whitelist of selectable column-header IDs. Defaults preserve existing
behavior: visible row headers, wrapping keyboard selection, all headers eligible
when `column_selection` is enabled (that global opt-in remains false by default).
`Some []` disables whole-column selection; `None` restores all eligible headers.
The whitelist contains at most 64 distinct IDs from the current schema. Clear or
update its references before removing a column with `Config.with_columns`.

Header eligibility affects whole-column selection, context and copy requests and
selection commands. Left/right skip ineligible headers, respecting Stop/Wrap;
Home/End select the first/last eligible header. An empty eligible set leaves
header navigation inert. It does not disable the column's cells, sorting, resizing,
movement or child controls. Row/cell eligibility still follows `Selection_mode`.
Row headers affect cell-selection modes; when row and cell selection are both
allowed, hiding row headers makes a repeat single click on the selected cell
select its row. This is the native table convention. Accessibility cell selection
remains an explicit cell selection and does not perform that pointer escalation.

Policy updates preserve the source and native table owner. Bonsai retains surviving
cell computations and repairs a now-invalid selection to Empty. Queued callbacks
from a previous configuration are ignored; controller commands are checked against
the current configuration. Existing schema-update behavior applies to transient
column geometry: native columns reconcile to the submitted schema. Accepted widths
and order belong in that schema, as for other table configuration updates.

The original table Config and Column wire layouts remain unchanged. Appended Op111
`Set_table_behavior (node, behavior option)` supplies bounded metadata; None clears
it to defaults. Core omits the extension for default configurations. Reconciliation
advances the mounted schema revision when the extension changes, including reset.
Native admission validates the final transaction snapshot, charges retained metadata
against the tree budget and requires that revision advance. Operation order within
a transaction does not change admission. Non-table attachment and unknown/duplicate
header IDs reject atomically.

Native callbacks retain revision-checked routes and deliver asynchronous intents.
No OCaml callback executes during rendering or native keyboard handling. Per-header
accessibility Select/Deselect actions use the same eligibility; Sort remains
independent. These additions do not provide structural tables, editable grids,
custom native header renderers, native striping or visible-column observations.
