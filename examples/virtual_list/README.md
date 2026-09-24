# Managed conversation example

Run `./scripts/gpuio exec dune exec examples/virtual_list/main.exe`.
For the automated real-window scenario, append `-- --self-test`.

The example uses `Gpuio_eio.List_paging.value` and `controls` with
`Gpuio_bonsai.Virtual_list.paged`. It begins with 200 messages and can prepend
100 older messages. Ready boundaries load near the viewport; the buttons also
show explicit request/retry controls. The history status distinguishes loading,
failure and end. The deterministic loader is local; replace its closure with
Eio filesystem/network operations using explicit capabilities.

A native multiline composer or the Stream button starts a simulated response.
The conversation scope owns both the producer and pager. Streaming updates one
existing message with `Pager.set`; they preserve order and do not rebuild history
metadata. Jump to latest resumes following after scrolling away.

Stars are application state outside the row computation and survive eviction.
Expanded details are transient row state and reset when the row leaves the active
set. Keep durable preferences in an application store; this example is in-memory.
Rows receiving delayed asynchronous results must guard completion effects with
the supplied `Managed_rows.Lifetime` or a conversation generation.

The self-test holds a page request open, moves away, streams into an offscreen
message, releases the page, checks the stable key/pixel anchor after prepend,
and jumps to the latest message. It uses the actual OCaml/Rust bridge and native
layout. Markdown, document resources and a complete chat product are later scope.
