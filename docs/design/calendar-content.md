# Calendar content slots

OCH-41, 2026-10-02. The retained content path passes local codec, native, driver and consumer-build
checks.
Physical macOS qualification remains a separate release gate.

## Ownership and public composition

The calendar keeps its native civil-date state, selection/range constraints,
keyboard cursor and action targets. Applications describe passive content for
explicit slots; no rendering closure crosses the bridge or calls back into OCaml
during layout. Unspecified slots keep the existing native label.

`Calendar.Slot` identifies previous/next, month/year toggles, Today/Clear, a civil
day, a month choice, a year choice, a displayed month heading or one weekday
heading within a particular month pane. Date/year constructors validate years
1..9999. Month/weekday headings are qualified by displayed month so simultaneous
panes do not share a retained content owner. Slots have stable structural keys.

The checked `View.Calendar_content` collection contains slots, optional
accessible descriptions and ordinary passive View content. It rejects duplicate
slots, interactive descendants and callbacks. `View.calendar` and the managed
date picker accept the checked collection without changing their existing
default behavior. Content changes retain the calendar owner, draft, focus,
selection and pending command identity. Content cannot replace native hit targets
or hide selected/today/cursor semantics. Decorative descendants are hidden from
accessibility; a description augments the native date/control name.

Example (ordinary passive views; the collection sorts by slot):

```ocaml
let content =
  let open Core.Or_error.Let_syntax in
  let%bind day = Calendar.Slot.day date in
  let%bind item =
    View.Calendar_content.Item.create
      ~slot:day
      ~description:"Two scheduled events"
      (View.column [ View.text "14"; View.text "2 events" ])
  in
  View.Calendar_content.create [ item ]
```

Pass the checked value as `View.calendar ~content`,
`Gpuio_eio.Calendar.view ~content`, or either managed `Date_picker` view's
`~calendar_content`. An empty collection and an omitted collection both render
native defaults. Edits to existing content preserve structural slot identity;
removing a slot restores its native label. Descriptions are explicit; changing
visible text does not infer or rewrite an accessible description.

The native renderer mounts each retained slot only in its assigned calendar
location. It excludes these wrappers from generic child layout, preventing
extra labels below the calendar. Rich descendants remain decorative to
accessibility, while native buttons keep their original name, role, action,
selected state and focus identity. Native selection borders remain outside the
clipped content. Content can use ordinary passive image/animation/progress views;
it cannot introduce another input owner or selection behavior.

## Bounds and wire representation

Op89 `Set_calendar_content (NodeId, metadata option)` attaches metadata to the
calendar owner. `None` requires no retained children; `Some []` is valid. Each
entry has one otherwise empty container wrapper with exactly one passive child.
Both metadata and child changes are admitted atomically. Existing tags remain
unchanged.

Metadata allows at most 1024 unique slots in strictly increasing slot order,
each optional description at most 1024
UTF-8 bytes, and at most 65536 total description bytes. Descriptions follow the
existing calendar label rules: nonblank, no ASCII control bytes. The retained
passive subtree keeps the existing 4096-node/128-depth aggregate limit, including
structural slot wrappers. Wire tags are Previous=0, Next=1, Choose_month=2, Choose_year=3, Today=4,
Clear=5, Day=6, Month=7, Year=8, Month_heading=9 and Weekday=10. Day uses
the civil ordinal 0..3652058; month choice uses 1..12; year uses 1..9999.
Headings use a month index 0..119987, and weekday adds Sunday=0..Saturday=6.
A wire item contains its typed slot and description;
its retained child wrapper occupies the corresponding ordered slot. The checked
View collection sorts supplied entries together with their content; the wire
requires canonical order for bounded binary lookup without an additional native
per-owner index allocation. Metadata
and child hierarchy are validated together in the final transaction tree,
including updates that only change descendants. Native heap accounting includes
the metadata vector and strings.

No application can issue a synchronous per-day predicate or callback. Supplying
content for an invisible date retains its description without rendering it;
removal/window close releases its resources. Theme changes and descendant-only
updates invalidate visible rich content. Native month navigation must render
the correct accepted content without requiring an OCaml frame callback.

## Viewport-based content loading

The existing snapshot month remains the keyboard cursor's month. Applications can
subscribe to [logical viewport observations](calendar-viewport.md), use
`Calendar.Viewport.dates` to load a bounded set of grid dates, and supply a sparse
content map. Missing entries keep native fallback. Independent subscription
identity and sequence let applications reject outdated asynchronous results;
selection revisions and pending picker confirmation remain unchanged.

The independent metadata fixture covers all 11 slot tags. Local admission,
reconciliation, driver, native rendering/lifetime and consumer-build results are
recorded in the [evidence](../evidence/calendar-content-och41.md). The authored
physical gallery walkthrough is unrun; local TestPlatform results do not replace
macOS qualification.
