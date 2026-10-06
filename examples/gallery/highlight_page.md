# Subtree search configurations and paired native observations

[highlight_page.ml](highlight_page.ml) and its [interface](highlight_page.mli)
compose a query editor, scoped Markdown source and ordinary text highlight scopes.
Read `Resources`, `summary`, acquisition/editor/state, `signature`/config/status,
match navigation, notebook scope and independent saved range. `B = Bonsai.Cont`
is graph construction, `E` deferred actions, `V` views, `D` scoped documents and
`H = Highlight` pure validated search descriptors. No service/network search runs.

After [setup](../../docs/development.md), from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section highlighting
```

Choose Find & highlight, search ideas, change case/whole-word, advance matches and
Add a paragraph. These are authored native checks, not newly run evidence;
[README](README.md) records platform acceptance. Native text/document highlighting is implemented in the current host and described
by the public contracts. No separate highlight capability is advertised. The
[recorded highlighting evidence](../../docs/evidence/subtree-highlighting-och41.md)
distinguishes prior local native/gallery checks from capability advertisement and
full release qualification. No capability gate disables this page's mounted scopes.
Constructing descriptors alone is still no proof of a painted result.

## Resource lifetime and reactive query state

`Resources.create` publishes a `Streaming` source containing Markdown, code and
literal ideas. `Preview_scope.acquire` owns it in a fresh gallery-highlighting
child of the window scope. `append` accepts one deterministic paragraph only,
marking its Boolean after `D.append` success; repeated clicks do not duplicate it.
It does not finish `Streaming` status. Local publication and native parser/paint
completion differ, per [Document](../../lib/eio/document.mli). Departure releases
this registration and clears observed/cursor/notice; a new activation starts anew.

`Editor.create` seeds ideas in one `Single_line` native placement. `B.toggle` stores
case-sensitive/whole-word=false and enabled=true; `B.state` holds cursor, paired
observation and notice. `let%arr` derives configurations/views from these changing
values. Query text uses the last `Editor.snapshot` or the initial fallback; this
page has no separate composition-free filter, so do not import the searchable
list's committed-query policy into this code. Native text/IME belong to the
[editor](../../lib/eio/text_input.mli); page departure destroys that lease and this
preview does not mirror its draft for future mount.

## Validated matching and configuration pairing

The `signature` contains query text, case sensitivity and whole-word policy. A cursor saved for another
`signature` resets effective active index to 0. `H.Query.create` validates a nonempty
literal of at most 4096 UTF-8 bytes without NUL; whitespace is meaningful. Unicode
scalar lowercase is not normalization/full case folding; whole-word checks
alphanumeric/underscore neighbors and matches cannot cross logical groups/newlines.
`H.Spec.create` adds palette-resolved appearance with radius 3 and `active_index`; a single
spec becomes Config. Disabled/empty query uses `Config.empty`, and invalid config
also falls back to empty while reporting validation text.

`observed` stores `(config, sample)`. Status/count are used only when the stored Config
matches current Config with typed equality, preventing an older configuration's
count appearing current. `summary` handles Pending, Ready with one spec, no matches,
invalid range/capacity/failure, and reports total versus stored painted-prefix counts.
Native epochs are local to one mounted scope, not globally comparable identifiers.
Next/Previous wrap against the accepted total; `on_update` also clamps an outgrown
active index when source changes. Setting active style does not itself execute a
search or certify a selected match is physically visible.
See [Highlight](../../lib/core/highlight.mli) for bounds and typed observations.

## Search projection, nested override and byte range

The main keyed `V.highlight_scope` wraps a heading, split styled id/eas pieces,
selectable text and the same-source Markdown View.document in a 230-pixel viewport.
The split row demonstrates ordinary projection across styled text pieces within
a logical group. A nested `Config.empty` explicitly excludes Private ideas rather
than inheriting the ancestor. Children keep identities while configurations change;
`V.highlight_scope` returns asynchronous observations, not an OCaml layout callback.

An independent scope highlights half-open bytes `[0, 6)` in 世界 · a bright idea: two
three-byte UTF-8 scalars. Range.`create` validates ordered endpoints, while matching
checks length/scalar boundaries; native documents are excluded from ordinary-text
explicit range projection. This saved range is independent of query/active cursor.

Trace: change query → reactive snapshot/`signature` changes → new Config derives →
old paired sample is ignored until matching native observation → Ready count enables
navigation → Next returns setter effect and changes `active_index`. Add a paragraph
publishes source data, leading to updated asynchronous counts; it is not a new
provider message or an assertion of immediate paint.

For adaptation, keep Config/Observation pairing and per-scope epoch ownership,
validate byte boundaries for saved passages and define a draft/composition policy
explicitly. Larger searches need bounded descriptors/worker failure handling,
not unbounded query strings. Native harness/pixel evidence and compilation test
different layers; no new platform acceptance is claimed by this guide.
