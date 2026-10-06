# A bounded transcript with a document outside transient rows

[chat_list_preview.ml](chat_list_preview.ml) and its
[interface](chat_list_preview.mli) compose a managed transcript. Read Source,
preview acquisition/editor/rows, Virtual_list.component renderer and controller
buttons. `B = Bonsai.Cont` is reactive graph code, `E` deferred actions, `V` views,
`L = Gpuio_bonsai.Virtual_list` manages row computations and `D` registers text.
This is a fixed 100-message fixture, not a paged backend or chat-send service.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section chat-list
```

Find the managed conversation in Presentation. History jumps to row 50; Latest
returns to the tail; Append grows row 100's Markdown. Draft is below the rows.
[README](README.md) separates authored native checks/macOS evidence/Linux GUI limits.
No harness or build was run by this documentation review.

## Source and draft own different lifetimes from rows

`Source.create` registers a `Streaming` `Text_source` with an OCaml code fence using
`D.create`. `Source.append` adds at most twelve deterministic Unicode paragraphs,
incrementing chunks only after `D.append` accepts. The cap does not finish status,
and local publication acceptance is not parsed/painted height. Preview_scope owns
a child window scope named gallery-managed-chat; departure cancels registration,
returning starts a new source/count. Notice resets on deactivation.

`Editor.create` seeds A draft outside the rows once in a `Single_line` controller.
It is rendered below the list, so scrolling/evicting rows does not remove it.
Leaving the page still unmounts its native lease; no mirrored application draft
is stored here for later remount. No row owns/cancels document acquisition or a
provider task. There is no automatic streaming Eio loop: Append is an explicit effect.

## Collection, managed computations and tail policy

`List_collection.of_alist` uses typed `Int` comparison for keys 1–100 and unit payloads,
with injective `Key.of_int` row identity. `B.return` provides that immutable collection.
`L.component` uses Estimated 140-pixel height, 180-pixel overscan, `max_active = 12` and
`Follow_tail_when_at_end`, bounded by a 560×360 viewport with maximum width 100%.
Only requested/pinned/prefetched rows have computations; the full logical collection
still has 100 entries. [Virtual_list](../../lib/bonsai/virtual_list.mli) defines native
measurement, budget and controller lifetimes; `max_active` is not a message-data cap.

`render_row` derives ID/source/palette/react injector with `let%arr`. Rows 1–99 are
fixed text, every third with a second line; row 100 mounts same scoped source handle
in Markdown Flow. Even/odd rows alternate Secondary/ Tinted and Start/End alignment.
Typed reactions dispatch a global count, not persistent per-row state. The renderer
has no async producer/local retained model and ignores its supplied row lifetime;
any future asynchronous row completion must use that lifetime guard.

Message/Header/Bubble are [stateless descriptors](../../lib/core/presentation.mli).
Absolute reaction controls do not increase measured row height, so row padding
reserves 12 px above, 28 px below and 12 px on either side. Native Flow changes can
invalidate measured row height. Being at tail permits native following as content
grows; after history navigation it remains at history until Latest, without
application code forcing scroll on every append.

## Effects, traces and limits

Final `let%arr` combines list output/resource/error/editor/count/notice. Errors or
Loading produce text; Ready mounts Output.view and borrows its controller.
History calls `scroll_to` with key 50 and offset 8; Latest calls `jump_to_latest`. Commands belong to
the current mounted list lifetime and obsolete controllers ignore delayed calls.
The label reports active rows/12, which is transient retention, not OS memory usage.

Trace: History effect scrolls to row 50 → native viewport requests a bounded subset →
old unpinned rows deactivate/reset → row 100's document source stays alive → Append
updates it even while offscreen → Latest requests tail → row 100 mounts current
source/height. The outside draft remains mounted throughout those list transitions.
Native screenshots/input measure the resulting behavior separately from resource
acceptance or active-count labels.

For another conversation, retain immutable message data and scoped producers outside
`render_row`, use stable membership IDs and change list generation when reusing keys
for an independent transcript. Guard/cancel actual row-owned async work if added;
preserve explicit tail policy and reaction spacing. This module is not a complete
persistence/selection/network implementation and has no independent pure test.
