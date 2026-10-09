# Managed-table renderer slots

OCH-41 implementation contract. This extends the retained table adapter, not the
fully mounted structural table. Rich-header submission/rendering is implemented
locally; see [evidence](../evidence/table-header-rendering-och41.md). Checked
header/body-row presentation now has local implementation/test evidence; see
[presentation evidence](../evidence/table-scoped-presentation-och41.md). Physical
qualification remains open.

## Submitted content and identity

Rich leaf/group headers are ordinary submitted Views. Rust renders them inside
the native header cells; it never calls OCaml from its delegate. Body cells keep
the existing managed-row/cell contract. Header content is a separate, bounded
retained set, independent of body viewport eviction and the active-cell budget.
All ordinary View/editor/asset/node/byte quotas still apply.

Each custom header has an explicit sibling `Key.t`, independent of its label,
column position or current group address. Stable keys retain native controls;
changing/removing keys retires them normally. No key is manufactured by joining
column strings or hashing labels. The caller owns any Bonsai computation producing
a header View. Source replacement still replaces the table's native mount.

Targets are either a stable column ID or a zero-based group level plus an exact
set of member column IDs. Group members are canonicalized by typed ID comparison;
display-order changes within the group do not change its target. Matching ignores
the label, checks the chosen level, and requires the complete group membership.
A partial overlap or a coincidental repeated label is not a match. The current
schema already validates contiguous groups, refinement and pinned boundaries.

Accept at most 320 slots (64 leaf columns plus four levels of at most 64 groups).
Reject duplicate keys, duplicate targets, missing columns/groups and invalid
levels/membership before generating wrappers. New wire metadata must be bounded
and canonical on decode as well as at public construction. Existing schema and
table Config representations remain unchanged.

## Native structure and behavior

Generated header wrappers are distinct from managed body row wrappers. List
admission must account for that explicit distinction; arbitrary children must
remain forbidden. Each wrapper has exactly one content View and belongs to one
table. Final-snapshot validation binds every target to the current schema and
rejects duplicate ownership atomically. Reset/removal releases the slot and its
resources. Cross-node validation and retained-byte quotas include the new metadata.

The native delegate needs exact group coordinates/membership, not just its label
and span. Extend the scoped extraction to provide that identity to the host while
preserving a default renderer. Pinned/scrolling pane geometry, column widths,
header heights, resize handles, sorting and header selection remain native.
Interactive descendants own their normal actions and must not accidentally trigger
the enclosing header gesture. Native focus/reveal must account for header controls
without pinning a body row. Hidden or horizontally culled content must not accept
stale pointer/accessibility actions.

## Header and row presentation

Upstream `render_header` and `render_tr` return base containers that the table
subsequently fills with native cells. Expose their presentation separately from
rich header content; do not replace the whole row with an unrelated View.
Application row styling is evaluated from active OCaml row data, before submission.
No synchronous native-to-OCaml callback or full-dataset style mirror is needed.

Native row/header geometry and semantic roles take precedence. A checked style
scope must reject layout, focus/input, scrolling and visibility overrides that
would invalidate table geometry or bypass its disabled state. Resolve theme
tokens normally. Clearing presentation restores defaults without replacing native
table/list owners or surviving cell models. Shared/per-column cell padding remains
owned by `Table.Appearance`.

`Table_presentation.Header` and `.Row` validate ordinary `Style.t` values with at
most 128 declarations across all states. Accepted fields are background,
foreground, border color/style, corner radii, shadows and font size/family/weight
and text decoration. Border widths, opacity, visibility, text layout, input policy
and all geometry/scroll fields are rejected. Children can override inherited text
properties normally. Native cell sizes remain fixed; large typography may clip.

Header accepts Base/Hovered/Pressed/Disabled. Row additionally accepts Focused and
Selected. Selected means whole-row selection, not a selected cell. Focused means a
focused row/descendant or the active row while keyboard focus belongs to the table.
Base presentation overrides striping, then native selection feedback applies.
Explicit states take precedence in this order: Selected, Focused, Hovered,
Pressed, Disabled. Pointer-disabled/disabled tables suppress pointer refinements.
Native selection/context outlines remain and filler rows have no submitted style.
The scoped adapter supplies native hover to a final-row hook so host refinements
merge before GPUI's single hover registration; default delegates retain feedback.

Core accepts an optional header presentation and unique keyed active-row
presentations. Unknown/inactive row keys are errors. Bonsai evaluates
`render_row_presentation` once per active row in its managed lifetime, independent
of the cell/column computations. Eviction/source replacement tears it down;
application tasks must live outside that scope. Styles resolve theme tokens during
reconciliation and emit only changed resolved values. Paint updates do not advance
schema revisions, replace scroll owners or recreate surviving cell computations.

## Integration and acceptance

Integrate checked descriptions through Core `View.Expert.managed_table`, Bonsai
component/paged constructors, reconciliation and appended protocol operations.
Keep schema/query/input generation fencing, bounded materialization and native
ownership explicit. A public gallery must exercise rich grouped/leaf headers,
interactive controls, row styling, reorder/resize, updates/reset and teardown.

Use paired codec fixtures, atomic malformed-admission tests, public reconciliation
and Bonsai lifecycle tests, real Host layout/input/accessibility checks and an
independent installed-gallery build. Verify header control focus/gesture separation
and culling, plus body anchors/selection and resource budgets. Record physical
macOS and required non-GUI Linux results separately. No source-level model or
TestPlatform result alone closes OCH-41 or OCH-17.

### Native header interaction detail

A retained header root can be partly visible while an interactive descendant is
fully outside the pane. Measure/gate that descendant as well as the slot; keeping
its sibling visible must not re-enable its own stale action route. Ordinary header
View recursion carries a native-only context for those per-control masks. Header
controls consume bubbling left-button down so their click/selection gesture cannot
arm the enclosing column drag. Do not prevent the control's own default focus or
selection behavior, or disable dragging on the remaining bare native header.
The implementation has deterministic Host tests for these distinctions, repeated
labels across levels, keyboard activation and teardown. Physical desktop and IME
coverage remains a separate release requirement.
