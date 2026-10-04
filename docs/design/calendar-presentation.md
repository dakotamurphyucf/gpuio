# Calendar presentation

OCH-41 implementation contract, 2026-10-02. Local integration is covered in the
[evidence](../evidence/calendar-presentation-och41.md); physical release acceptance
remains separate. This extends the single native calendar owner.

`Calendar.Appearance` supplies 1..12 visible months and bounded cell height, gap,
month gap, padding, corner radius and outline width, plus theme-resolved selected,
hover, today, focus and muted colors. Default appearance preserves one month and
the previous geometry. It is passed to Core/Eio calendar views and the Eio date
picker view. Appearance updates retain the selection, cursor, focus and native
lease. Ordinary outer styles continue to control the container.

A native display anchor owns the first visible month. The snapshot's `month`
continues to identify the cursor month, preserving the existing wire invariant.
The anchor stays put while the cursor moves among visible months. Moving before
or after the visible span shifts it just enough to reveal the cursor; all spans
are clamped to civil years 1..9999. Count changes retain the first month when
possible while keeping the cursor visible. Header arrows move the first pane by
one month. Explicit Show_month aligns the requested month with the first pane
when the civil boundary permits, even when the cursor already belongs to it. No application callback runs in paint.

Day mode displays a bounded set of consecutive month panes. Each has its own
month/year label and six-week grid. Adjacent-month cells that belong to another
visible pane are blank, so each visible day has exactly one interactive/AX target
and only one active descendant exists. Leading/trailing days outside the visible
span remain available for cross-month navigation as in the original calendar.
Months/Years mode remains one chooser for the cursor month. Returning to Days
reveals it in the retained span. Keyboard day/week/page/year navigation and range
constraints continue to use the shared owner. Selecting in another pane never
starts an independent selection.

Multiple month panes wrap with available width and have a minimum width derived from
seven cells plus gaps. A single month retains the existing flexible outer width.
Applications size the outer container or use an ordinary
scroll view. Months do not become arbitrarily tiny to fit an unbounded count.
Presentation changes do not trigger timers or a second OCaml round trip per pane.

The new optional bridge presentation operation is fixed-size and validated on
both runtimes. None resets defaults. Invalid kinds, counts, colors or dimensions
reject atomically. Tree allocation accounting includes retained presentation.
