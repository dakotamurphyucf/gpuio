# Command palette search policies

OCH-41 local implementation and scoped [macOS evidence](../evidence/palette-policies-och41.md); broader catalog/release acceptance remains open.

A palette owns a bounded native query and a captured document editor target.
`Command_palette.Config.create` references the enclosing command registry; it
never copies callbacks into the query widget. Selection rechecks current command
availability, generation, focus scope and captured editor before invocation.

The optional search policy defaults to `All_terms`: every whitespace-separated
query term must occur in the Unicode-lowercased label, ID or a keyword. `Substring`
compares the entire lowercased query with a label or individual keyword, matching
the pinned catalog's basic substring policy; it does not search command IDs.
`Unfiltered` keeps declared command order regardless of query. These are literal
matches, without fuzzy ranking, case folding or Unicode normalization.

`~searchable:false` omits the query field and bypasses filtering while preserving
its text and native identity for later configuration changes. The palette scope
receives keyboard focus in this mode. Hiding an active query ends its composition;
reenabling transfers focus back only when the palette still owns focus. Neither
transition steals focus from a newer overlay or recaptures the document target.

Escape defaults to dismissing the chooser. `Clear_query_first` clears a nonempty
visible query first; a later Escape dismisses. An active IME composition consumes
Escape before either policy. A hidden query never requires an extra Escape.
Arrow/Page keys skip disabled commands; selection is retained by command ID while
it remains available and is otherwise repaired to the first enabled match.

Keywords refer to unique IDs in this palette's declared commands: at most 1024
entries, at most 64 words per entry, each nonblank UTF-8 without NUL and at most
4096 bytes. Blankness follows `Core.String.strip`, matching existing command
metadata: non-ASCII spaces remain literal text. IDs are bounded to 256 bytes. Keyword text and repeated ID references
share the existing 256-KiB metadata budget with the label, placeholder and command
references. The query remains bounded to 4096 bytes. Retained-memory accounting
includes the native lowercase keyword index; updates rebuild that index once.

Protocol operation 125, `Set_palette_options`, appends an optional configuration
without changing the existing palette record encoding. `None` resets defaults.
The decoder checks counts and text budgets before allocating unbounded payloads;
transaction validation checks known command references and the shared budget.
Invalid transactions do not partially alter the mounted tree. Reconciliation
sends policy-only changes separately, preserving the palette node and owner.

This addition does not close the remaining richer palette presentation/query
controller or native OS popup work in the command/menu catalog review.
