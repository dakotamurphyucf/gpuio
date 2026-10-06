# Search results produced by an Eio task

This component opens a modal workspace search. The query stays responsive while
an application task produces results. It demonstrates query fencing: an old
request cannot replace the results of a newer query or a closed palette.

```sh
./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Feedback → Search beyond the current view → Search workspace**. Enter a
query and confirm a result to update **Opened …**. `missing` produces an empty
result; `slow` delays the mock response. Change the query during that delay to
exercise cancellation. These are deterministic demonstration responses, not a
network service. Native macOS behavior has scoped qualification; this guide does
not claim Linux GUI or screen-reader acceptance.

## Source map and state

Read the [interface](external_palette_preview.mli), `Candidate` and then
[`component`](external_palette_preview.ml). The [Feedback page](feedback_page.ml)
passes the search function down from [`application.ml`](application.ml). There,
`search_palette` captures the Eio monotonic clock, sleeps for 0.18 seconds (0.8
for `slow`) and returns at most two labels. It performs no filesystem/network
access. [`Preview_scope.acquire`](preview_scope.md) owns the component's child
scope. `Palette` supplies gallery visual helpers, not the native query engine.

The Bonsai model holds open/closed state, an optional `Candidate`, a status message
and the selected result text. A candidate contains a request serial, the native
query snapshot it answers and labels. Its command IDs include the serial and row
index, so commands from different responses are distinct. `Candidate.equal`
compares the serial because a candidate is immutable and each request gets a new
serial; mutating labels under an existing serial would violate that assumption.

Three refs track the running task, request serial and latest observed query.
They are constructed with the component graph, outside `let%arr`; they are private
task coordination, not reactive display state. The model remains in Bonsai.
Native query text, composition and highlight live in GPUIO and are observed through
`Command_palette.Snapshot`.

## Follow a query and its result

1. Native editing emits an asynchronous snapshot to `observe`. The controller
   records it, and `Snapshot.same_query` distinguishes query changes from other
   observations. An unchanged query starts no task.
2. A changed query cancels the previous task and advances the serial. While IME
   composition is active, the example clears the candidate and waits for accepted
   text. Otherwise it marks the native chooser loading, guarded by the snapshot.
3. `Scope.start` executes `search` in the page-owned Eio scope. The `current`
   predicate checks both serial and query before starting and before accepting a
   result. Cancellation is useful, but these identity checks remain necessary.
4. A successful result becomes a candidate. `let%arr` constructs registry commands
   and passive rich rows for those IDs. `Search.External` means native filtering
   does not reinterpret the application's result order.
5. `B.Edge.on_change` reacts to the new immutable candidate. In GPUIO's accepted
   View lifecycle, those definitions are mounted before `publish_results` installs
   their order. Publication checks the original query again. Stale/closed results
   are ignored; other command errors are shown in the status message.
6. Selecting a result invokes its command, updates **Opened …**, and dismissal
   cancels work, resets the controller and clears the modal/candidate state.

View acceptance, successful result publication and physical frame presentation
are separate events. Publishing IDs before their definitions are accepted is not
equivalent to the sequence above. The
[external-results contract](../../docs/design/palette-external-results.md)
explains the two-stage protocol.

## Lifetime, failures and adaptation

Leaving the page runs the Bonsai deactivation hook and cancels the child scope.
Closing the modal also retires its request; rows never own producer tasks.
Late completions fail the `current` predicate. A search error stops the loading
indicator through a guarded command and puts the error in the footer. The current
example does not provide debounce, retries or persistent remote caching.

To use a real search service, replace the application-supplied `search_palette`
function with an Eio implementation that captures explicit network/filesystem
capabilities. Keep cancellation effective and bound result count and label bytes
to the [registry](../../lib/core/command.mli) and
[palette](../../lib/core/command_palette.mli) limits. This two-result demo is not
an unbounded remote-results adapter. Add application-owned stable result IDs if
selections must survive multiple queries; the demo's serial/index IDs identify
one response only. Do not move I/O into `let%arr`, paint callbacks or row renderers.

For a persistent local chooser with no producer task, read the
[embedded palette example](embedded_palette_preview.md). The
[controller interface](../../lib/eio/palette_controller.mli) describes command
errors and the [scope interface](../../lib/eio/scope.mli) defines task cancellation.
