# Native container queries (OCH-26)

Status: implementation contract; not yet a mounted feature or advertised capability.
Read the live OCH-26 ticket and the platform policy in AGENTS.md for acceptance.

## Public model

`Container_query.Range` is an abstract half-open interval of logical pixels:
minimum inclusive, maximum exclusive. The default minimum is zero and an omitted
maximum is unbounded. Bounds must be finite, nonnegative and strictly ordered;
`Range.all` accepts every nonnegative assigned size. Width and height intervals
combine with AND in `Predicate`. Matching uses the actual assigned native logical
size, after GPUI layout/scale rounding, without rounding the declared threshold.

A `Rule` pairs a predicate with a stable `Branch_id`. A `Config` contains a required
default branch and up to 32 ordered rules. The first matching rule wins; otherwise
the default wins. Overlaps are intentional and deterministic. At most 16 distinct
branches may be referenced. IDs contain 1–128 UTF-8 bytes without NUL. A pure
reference selector supports tests and application reasoning, but does not perform
native layout or trigger OCaml callbacks from Rust.

The intended `View.container_query` mounts a config plus one supplied view per
referenced branch. Branches use stable IDs as reconciliation keys, independently
of rule order. The default is a real supplied presentation, not an empty implicit
fallback. Missing/duplicate/extra presentations are rejected before submission.
Each supplied view can contain ordinary styled containers or entirely different
layouts. This first contract uses bounded supplied presentations; it does not add
CSS selectors or independently mutable native copies of OCaml style trees.

## Assigned size and layout ownership

Use the pinned GPUI container-query primitive: it requests layout without its
contents, receives the assigned size, then lays out the selected child as a root
inside that size during prepaint. Native selection and child layout happen in the
same layout cycle, without an OCaml round trip. The container defaults to filling
the parent's offered area; normal outer styles can constrain it. Applications
must supply a meaningful parent/explicit size. Auto/intrinsic sizing cannot make
the selected child size its own query container. Child overflow does not feed
back into query selection. Nested queries receive their own assigned sizes.

The outer query's style is unconditional. Presentation styles affect the selected
inner root. Width/height of a branch never change the outer query's measured size.
Resizing a split pane or window selects natively; changing display scale uses the
resulting logical assigned size, not a separate physical-pixel breakpoint rule.

Pinned source: `crates/gpui/src/elements/container_query.rs` at GPUI commit
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. Its Rust render callback stays entirely
inside the native adapter. It is never an FFI callback into OCaml.

## Retention, focus and resource ownership

All supplied branches remain in the retained protocol tree and in the supplied
Bonsai computation. Selection changes native visibility only. Editors retain
buffer/history/selection, stable keys retain component identity, and normal
window/session resource quotas continue to include hidden branches. Hidden
branches do not paint, expose accessibility nodes or accept pointer, keyboard,
command or accessibility actions. Native timers/work follow each component's
hidden-state policy. Per-query branch and rule bounds supplement existing tree,
resource and animation quotas; they do not bypass them.

When selection hides the focused branch, cancel active pointer/drag/menu state,
release ineligible focus, and use normal eligible-focus fallback in the selected
presentation. Do not transfer buffer selection or IME ownership to a different
editor merely because it occupies a similar position. Do not restore focus on
every resize when the same branch remains selected. Nested focus scopes and
deferred overlays must obey current native query visibility.

A native selection does not activate/deactivate Bonsai lifecycle hooks or cancel
application Eio tasks. If an application wants to pause application-level work,
it may react to the asynchronous selection observation and explicitly scope that
work. That action follows selection; it cannot participate synchronously in
layout. Unmount/window close releases all branches and query state normally.

## Observations and bridge limits

The planned appended wire config carries a positive admission generation, ordered
branch IDs, a default index, and bounded index-referencing rules. Native decoding
bounds list/string lengths before allocating and independently validates all
ranges, indices, unique names and generations. Config changes advance admission;
branch identity remains its stable ID. No capability is advertised by the pure
schema alone.

Emit an optional typed selection snapshot after actual paint: initial selection,
branch changes, and the first painted selection of a changed config. Include
config generation, a monotone selection sequence, selected branch and assigned
logical width/height. No observation is emitted for every resize within the same
branch or for speculative/unpainted selection. Reconciliation rejects stale
window/node/handler/revision/config/sequence data and invokes the latest closure.
The native mailbox keeps bounded delivery under existing overload policy. No
observation callback controls the native branch decision.

## Acceptance work

Independent OCaml/Rust fixtures, bounded malformed-input checks and pure tests
cover defaults, overlaps, width/height combinations, fractional boundaries and
invalid sizes. Actual native/public tests must cover below/equal/above thresholds,
resize/scale, nested queries, stable editor/branch identity, focus and accessibility,
overlays, hidden timers, Bonsai lifecycle distinction, disposal and idle behavior.
Check geometry without an OCaml acknowledgement between size and selection.
The OCH-46 chat showcase will integrate responsive conversation/inspector layouts.
Consolidated hosted macOS/Linux gates and merge remain required; full Linux GUI
acceptance follows OCH-17, not compilation alone.
