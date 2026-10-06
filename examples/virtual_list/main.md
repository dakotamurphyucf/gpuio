# How `main.ml` separates conversation work from active rows

[README](README.md) · [Source](main.ml) · [Dune](dune)

The program holds 200 messages initially, can prepend 100 older messages, and
streams a simulated reply into existing message 300. It is not a network chat
client: both loader and producer construct local data.

## Data, scopes, and native layout

`Collection.of_alist (module Int)` validates the keyed collection, initially IDs
101–300. `row` constructs a two-line message; `key` converts its integer identity
to a validated `Gpuio.Key`. Keys must be stable and injective.

`run` creates a child `conversation` scope under `App.scope app`. `Pager.create`
owns history loading in that scope, with older boundary `More (Some "101")` and
newer boundary `End`. The ordinary loader sleeps 0.4 seconds and returns IDs
1–100; the test holds it on a promise. Producer work belongs to the conversation,
so scrolling a row away does not cancel a history request or simulated reply.
See [pager](../../lib/eio/list_paging.mli) and [scope](../../lib/eio/scope.mli).

`Virtual_list.Config.create` validates estimated 110-pixel height, 200-pixel
overscan, a 32-row active budget, and `Follow_tail_when_at_end`. The viewport is
bounded to 650 × 380 logical pixels. Native layout measures variable-height rows
and requests an active set asynchronously; OCaml does not run inside Rust layout
callbacks. The full collection still consumes application memory even though
only bounded row computations are active.

## Bonsai graph and two state lifetimes

`B` means `Bonsai.Cont`. `Pager.value` provides a reactive snapshot;
`B.return (Pager.controls pager)` provides constant generation-checked effects.
`Virtual_list.paged` connects these to native demand and a `render_row` graph.
Its result is a reactive `Or_error` output with a view, controller, viewport, and
active-row count. This example unwraps errors with `Or_error.ok_exn`.

`B.state Int.Set.empty graph` stores stars outside row computations. Each row's
`B.state false graph` stores expansion inside its transient graph. Opening
`B.Let_syntax` enables `let%arr`; row bindings read current identity, payload,
expansion, and star state to derive content. Button setters create effects.
Click Star: native delivery runs the setter, the parent set changes, and the
row view changes. Evicting that row preserves its star. Click Expand: only its
row state changes, its minimum height increases, and native measurement updates.
Eviction resets expansion on the next visit.

The supplied row lifetime is unused here because rows start no asynchronous
work. When adding delayed row work, guard completion with that lifetime; storing
a preference inside the row graph would give it the wrong lifetime.
See [virtual list](../../lib/bonsai/virtual_list.mli).

## Composer and streaming trace

`Editor.create` mounts a native multiline editor with `submit_on_enter:true`.
Its `on_submit` returns an `E.of_thunk` that calls `stream_response` with the
submission's text. Native editor draft/selection/IME state is not a Bonsai string.
The button starts the same producer with a fixed prompt.

`stream_response` rejects overlap using `streaming`, then starts a conversation
fiber. Every 20 ms it pushes a progressively longer string to a
`Gpuio_eio.Stream` of capacity 16. The stream delivers batches as UI effects;
`on_batch` keeps the last payload and calls `Pager.set ~key:300`. That point
update preserves order and history metadata. Bonsai updates a visible row, or
retains the new data until an offscreen row becomes active. `Exn.protect` clears
the overlap flag even if cancelled. The producer marks `stream_done` before
queued delivery necessarily drains; it is test bookkeeping, not a render barrier.
See [stream](../../lib/eio/stream.mli).

Load older/Retry older use snapshot-generation controls. Jump to latest sends a
controller effect, returning to native tail following. `B.Edge.after_display`
records the output for diagnostics, rather than performing I/O during rendering.

## Commands, test, and adaptation

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/virtual_list/main.exe
_build/default/examples/virtual_list/main.exe
_build/default/examples/virtual_list/main.exe --self-test
```

`App.run` opens a 700 × 650 window and owns shutdown. The graphical self-test
uses `perform` to enter effects through a scope completion callback and bridge
results to Eio promises. It checks the active budget, holds an older page, scrolls
to key 200 with offset 17, streams offscreen, releases the page, checks 300 rows
and the retained anchor, then jumps to the tail and awaits a native frame.
Success prints `MANAGED_LIST_PASS` and shuts down. Waits have 15-second limits
and the final frame has a 5-second limit. This checks bridge/layout behavior,
not real composer typing, IME, every eviction preference, or Linux GUI acceptance.

Replace local loaders with explicit Eio filesystem/network capabilities captured
outside graph evaluation. Bound stream payload bytes as well as value count,
handle rejected pushes, keep durable data outside row graphs, and use pager
reset/generation policy when replacing a conversation. No guide claim here
substitutes for [platform qualification](../../docs/platform-release-policy.md).
