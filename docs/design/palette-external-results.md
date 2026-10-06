# External command-palette results

Implemented under OCH-41 with [local native/OCaml and installed macOS evidence](../evidence/palette-external-results-och41.md). Broader catalog and release qualification remain open.

`Search.External` retains the native query but presents only explicitly published
results. The initial result set is empty. `Command_palette.Results.create` and
`create_entries` define an ordered, bounded set of unique command references with
optional groups, headings and separators. References must belong to the mounted
palette config and resolve in its enclosing native command registry. The existing
registry remains the authority for labels, enabled state, native edit targets and
application callbacks. Rich row slots retain their configured command indices,
independent of published order. No OCaml callback runs during native rendering.

`Gpuio_eio.Palette_controller.publish_results t ~expected results` captures the
snapshot's window/node/observer and query identity. Native admission refreshes
from the actual editor revision, including unobserved edits and typing away and
back, before checking the query. The operation validates all command references,
installs result order/layout and clears loading together. Missing/invalid command
references apply no result change. Composition rejects publication. A covered
palette may accept results without taking focus. Replies acknowledge application,
not paint. Core `Command.Publish_results` uses the same path; App always attaches
a query fence, and raw wire publication without one is invalid.

Surviving enabled selection is preserved; otherwise normal palette selection
chooses the first enabled row, unless the caller explicitly cleared selection.
Publication never activates a command or changes query text, caret, undo or marked
text. A same-count reorder still advances snapshot sequence, while preserving
query identity. Results bypass built-in text matching; the producer determines
relevance and ranking. Native registry eligibility remains enforced at activation.

A native query edit invalidates published results immediately when native state
is refreshed, before queued row activation can use them. The palette displays no
external rows until new results are admitted. Loading remains independently
controlled while the application starts or cancels work. Configured command-ID
changes, observer retirement/replacement, leaving External mode, closing and
unmounting retire the result payload. Returning to a local search policy restores
its ordinary matching behavior.

## Staging dynamic commands

Applications can stage fetched records as new registry definitions and rich View
rows, then publish their references. These are two distinct operations. The final
publication atomically controls visible order/selection/loading; it does **not**
make separate registry-label or rich-View mutations transactional.

Do not update Bonsai state and immediately assume a native command is mounted:
App submits queued commands before cycling the next View candidate. Instead,
attach publication to the staged candidate's accepted View lifecycle, such as
`Bonsai.Edge.on_change` keyed by the result-request identity. GPUIO's driver
captures lifecycle effects with the exact submitted candidate and triggers them
after native `Accepted`. While acknowledgement is pending, later model actions
remain buffered. An unchanged native View accepts locally. This requires no
render callback and continues for occluded windows.

The gallery's `External_palette_preview` demonstrates this sequence with a
page-owned Eio scope, changing result IDs, rich rows and a delayed producer. New
queries cancel the previous task; an additional request identity and query check
suppress stale queued producer completions before staging. The native publication
fence still handles typing between staging and admission. Same-query retries need
this producer identity: query identity alone cannot order two jobs for identical
text. No asynchronous search is started implicitly by the core palette API.

## Bounds and wire representation

Results accept at most 1,024 unique command IDs, 1,024 layout entries, and 256 KiB
of combined metadata including the seven-byte internal validation label allowance.
IDs/groups/labels use the existing palette UTF-8, blankness and NUL rules. Group
indices form a canonical ordered traversal covering each result once. Commands
must also fit the mounted palette/registry's existing bounds. External mode
reserves 2 MiB per palette at tree admission for its maximum retained result
payload and projections; replacing results cannot bypass the native tree budget.
This is a conservative accounting reservation, not an eagerly allocated buffer or
an assertion about whole-process memory.

The unpublished paired protocol appends search policy tag 3, command tag 5
`Publish_results` (ordered IDs plus optional grouped layout), and error tag 9
`Invalid_results`. Existing snapshot representation is unchanged. Both runtimes
must be built from the same release revision. Strict payload bounds, independent
byte fixtures and native behavior tests remain required; no backwards-compatible
ABI claim follows from appending tags.
