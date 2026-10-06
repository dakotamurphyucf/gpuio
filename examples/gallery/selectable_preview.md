# Search ownership, membership targets and transient selectable rows

[selectable_preview.ml](selectable_preview.ml) and its
[interface](selectable_preview.mli) combine window-owned asynchronous search with
a Bonsai Selectable_list. Read Entry/initial/matches/create, source/layout/interaction,
query/slots/config/actions, row renderer, final command helpers and controls.
`B = Bonsai.Cont` constructs reactive computations, `E` later actions, `V` views,
`L` selectable-list state, `P = Gpuio_eio.List_search` scoped producers and
`C = List_collection` immutable application data. These are distinct owners.

After [setup](../../docs/development.md), from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section selectable-lists
```

Choose Collections → Searchable list. Try remote/offline/slow or the supplied
buttons; there is no actual remote server. [README](README.md) gives authored
native/consumer coverage and deferred Linux GUI limits. No build/harness ran here.

## Create producers outside Incremental graph evaluation

The abstract `t` holds one `(int, Entry.t, Int.comparator_witness) P.t` search owner
and a `fail_next` reference. `initial` creates four section rows with negative keys and 250
records per group with positive IDs 1–1000, 1004 loaded rows total. Labels/details
are deterministic English fixtures. Negative rows are disabled decorations; every
positive key divisible by 11 is an unavailable option. group maps fetched key 10001
to a fifth section, otherwise positive IDs by groups of 250. matches skips section
rows, performs case-insensitive title substring matching and rebuilds ordered
section headings before matching IDs.

[Application](application.ml) creates this owner before component graph evaluation,
with a gallery-search child of `App.scope` and the monotonic Eio clock. It cancels
that child when the exact window scope closes; open failure also cancels it.
The producer can survive page/row unmount while that window lives. create sets
`max_loaded = 2048`; [List_search](../../lib/eio/list_search.mli) initially exposes all
loaded records Ready without starting work. Default debounce is 150 ms.

The injected producer reads an immutable Request snapshot, sleeps 0.18 seconds
(or 0.8 for slow), then returns `Page { upsert; visible }`. offline fails once when fail_next
is true; retry uses empty-query results. remote upserts heading -5 and 10001 then
shows just those, retaining omitted loaded items for hidden selections. slow also
matches empty query after its delay. A page is validated atomically; duplicate or
missing visible keys cannot partly install. Superseded tasks cancel and epochs
fence queued completions; no implicit loaded-data eviction occurs. 2048 bounds
record/result metadata, not arbitrary payload bytes or all process memory.

## Separate source identity, visibility, interaction and selection

component observes `P.value`. source derives `Snapshot.items`; identity and visible
are separately cut off with phys_equal to reuse layout when point payloads change.
`List_selection.Catalog.create` combines membership identity, exact visible keys and
disabled IDs; `List_rows.Layout` marks negative keys as Decorations. These IDs are
collection membership, not native node indices or row position. [List_collection](../../lib/core/list_collection.mli)
keeps surviving `Item_ref` identities through point updates; removal/reinsert or
independent source retires old targets even when an integer key is reused.

`L.Interaction` uses search epoch, `Multiple` initially, or `Single` after toggling, Wrap boundary, busy and
stale-disabled. Pending/failed/cancelled search retains previous visible results
but marks them stale and disables list input; busy is a separate observation.
Hidden committed selections survive filtering, while cursor and confirmation are
separate intents. Mode/query/policy changes fence captured input as documented by
[Selectable_list](../../lib/bonsai/selectable_list.mli). Confirm does not imply a
new selected set, and source replacement/unmount retires a controller.

## Native search editor and direct query slot

`Editor.create` makes a `Single_line` native query with `clear_on_escape`=false. A
stable `query_key` marks its view as the list's one direct query Input in before;
after contains nonshrinking status text. Changed with no composition calls
`P.set_query` using the rendered source token; stale tokens ignore old editor events.
Submitted/Search_changed/composing observations do not start production. IME stays
native, and [Editor](../../lib/eio/text_input.mli) `initial_text` is a mount-only seed
from current query, not live draft replacement. Search work never runs in the
renderer or mutates Bonsai from another domain.

Status covers Ready/empty, Debouncing, Loading, Failed, Cancelled and Closed.
Pending/cancelled previous rows remain present rather than replacing list/editor
with a spinner. Button `replace_query` first checks source token, explicitly replaces
native text with selection End/undo Record, then rechecks source on reply. Command
replies do not invoke ordinary event callbacks, so it calls set_query itself.
If text already equals current query it refreshes; ordinary set_query with same
text is a no-op and would not retry. Retry/Cancel use captured search epoch and
ignore obsolete controls. Try recovery resets fail_next before selecting offline.

## Bounded rows and current-target actions

Vertical config uses fixed 64-pixel row height; horizontal uses fixed 240-pixel
width. Both set `max_active = 16` and use a 360-pixel-high viewport. Config changes preserve surviving
row state/controller rather than replacing collection. [Managed rows](../../lib/bonsai/managed_rows.mli)
retain only the requested/pinned subset; logical data is still O(loaded records).
The optional scrollbar description comes from Collections' scrollbar preview.

Each `render_row` has local Inspect count of 0. Decorations are Heading 3; options show
label/detail and disabled text. Inspect wraps `E.Many`(local setter + Secondary
confirm target) in `Lifetime.guard`, preventing effects from an evicted visit changing
a revisited row. Local counts reset on eviction; selected refs/application data
live outside those row models. Ordinary accepted native callbacks also have native
generation fencing, distinct from application membership/source/row guards.

`on_action` resolves a target key against **current** `P.snapshot` data before reporting
Opened/Preview/Context. Those notices simulate intent; they do not open a file or
create the advertised context menu actions. Cancel clears only cursor, preserving
selection and search text. Selected summary reports count and first three labels.

Update current detail checks cursor membership in latest collection and uses `C.set`
followed by `P.update_source ~refresh:false`. This declares matching/order unchanged,
keeps visibility/epoch and makes newer detail win over an older pending upsert.
New collection passes fresh initial(), fences old source work/selection targets
and starts a fresh search. It is not a cosmetic clearing of one row.

Concrete trace: select IDs in multiple mode → type remote with no composition →
old results become stale/disabled while scoped producer waits → valid page merges
fetched records and new visible heading/ID → hidden selections remain in original
collection → clear query restores eligible original results. A slow superseded
reply cannot replace the newer epoch. Inspect count belongs only to that row visit.

For real search, inject explicit service capabilities into create, validate/bound
records and payloads, and preserve cancellation/epoch/source tokens. Keep producers
and selected preferences outside rows; guard actual delayed row completions. Do
not describe this fake remote path as network acceptance or `max_active` as a bound
on retained collection bytes. Public search/selection tests and physical native
harnesses establish different behavior from this source review.
