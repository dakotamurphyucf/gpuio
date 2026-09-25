# Agent workspace composition

The OCH-16 reference application demonstrates the OCH-14 document and OCH-15
workspace APIs alongside the existing managed lists, native editor and command
system. See the [runnable example](../../examples/agent_chat/README.md).

## Ownership and state

One `App.run` owns the native application and OCaml UI domain. Three explicit
conversation stores are shared across up to four window drivers. Each window
owns its own Bonsai graph, command registry, theme, search, selected tabs, editor
controllers and list views. A conversation owns its pager, response documents
and accepted producer tasks. The stores are UI-domain-only; Eio fibers on that
domain schedule the deterministic backend without invoking OCaml during layout.

A virtual row only observes a message and its document handle. Scrolling it out
of view releases its native presentation but never cancels a response. Closing a
tab hides a retained panel; reopening restores native editor identity, draft,
selection and reading anchor. All three seeded panels remain mounted per window,
which makes the retention bound explicit. Closing a window cancels its scope and
releases all its panels; accepted conversation producers survive while the
application remains alive. Last-window close ends this demo and its app scope.

Submission first captures a native `Text_input.Submission`. The window owns a
cancellable acceptance task. After acceptance, the new response document and
producer belong to the conversation scope, and the callback attempts
`clear_if_unchanged` with that exact submission. A newer editor revision is kept.
Window closure before acceptance suppresses the callback and creates no response.
Cancellation after acceptance keeps the valid partial document; retry resets its
source generation and reuses the response row, without duplicating the user turn.

The pager begins at a known latest boundary and supports older-history loading
concurrently with appended live rows. `List_paging.append` preserves collection
identity, cursors and in-flight history. Each list owns its own follow-tail state
and anchor, even when another window views the same conversation.

## Native boundaries and API refinements

Rust owns text/IME/selection/undo, scrolling, split geometry and document layout.
Bonsai receives asynchronous observations and typed intents. Streaming publishes
coalesced document changes rather than rebuilding a transcript per chunk.
Markdown and highlighting use bounded native background workers; limits and
fallbacks are specified in [documents](documents.md).

Integration added three general composition operations:

- `List_paging.append` appends only when `After = End`, atomically rejects
  duplicate IDs, and preserves an outstanding older-page request.
- `Text_input.submit` captures current native text and invokes the configured
  handler once. Both Send and native Enter follow the same application contract;
  an older Bonsai observation cannot become the submitted prompt.
- `Text_input.read_snapshot` reads current native text and marked composition
  without changing focus or submitting. It works for hidden/disabled retained
  editors. The close guard uses it to inspect all drafts, including composition,
  rather than trusting an after-display observation that may lag the last input.

Ordinary nonselectable `View.text` exposes native Label semantics, so status and
dialog text are visible to accessibility clients. Selectable text retains its
existing selection/accessibility element. Hidden panel descendants cannot accept
focus or submit; retained native snapshots remain readable for lifecycle guards.

A close guard obtains current snapshots and, if any draft is nonempty or composing,
returns an asynchronous decision from a modal dialog. Duplicate close requests
coalesce in the window runtime. Force-close is deliberately separate. This is an
in-memory demo policy, not a durable save protocol. Shared application quit and
platform capabilities are specified in [windows](windows.md).

## Bounds and application responsibilities

The reference app retains at most three conversations, four live windows, three
panels per window, 200 history rows, 64 responses and eight 64-KiB attachments per
conversation. Lists mount at most 32 ordinary rows, with the library's documented
pinning behavior. Prompts are limited to 16 KiB. Fake backend chunk size, delay,
acceptance delay and failure position have validated bounds and pure expect tests.

File selection returns native path bytes. Only explicit Eio capability-backed
reads follow; no filesystem access is delegated to Markdown. Navigation is an
application intent, and tool cards are display data. There is no provider SDK,
credential handling, process execution, persistence, docking or plugin host.
Those are application concerns or later milestone capabilities. Linux remains a
build/unit gate; native Linux application validation is deferred to OCH-17 under
the owner's accepted development policy.

## Showcase visual design

The owner requested that this be a polished showcase, not merely a functional
test harness. The reference application uses a semantic dark/light palette,
original monochrome SVG icons, restrained lilac accents, a compact sidebar and
tab strip, a bounded reading column, and a prominent rounded composer. Tool
artifacts start collapsed and expand into native selectable monospace source.
Native tooltips label icon controls; keyboard focus, progress, failure feedback,
command palette and close dialog are styled and inspected in actual windows.
Simulator controls are available without dominating the conversation. Resource
registration starts with the first native window observation, after negotiation.
No webview, raster mockup or screenshot is used to render this interface.
