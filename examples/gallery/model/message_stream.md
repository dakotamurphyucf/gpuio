# Message stream model: bounded stable rows and one growing latest value

[message_stream.ml](message_stream.ml) and its [interface](message_stream.mli)
provide a pure immutable history model for the gallery's managed message list.
Despite the name, there is no network stream, task, timer or I/O cancellation here:
buttons dispatch actions that add mock rows or extend a mock response. Bonsai and
the native managed list belong to [collections_page.ml](../collections_page.ml).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Lists, trees & tables → Message list**. Add Earlier history/New message,
grow/reset the latest response and follow tail or read earlier entries. There is
no independent executable, asset or model-specific diagnostic flag.
[Development](../../../docs/development.md) covers toolchain prerequisites and the
[gallery README](../README.md) records scope/platform limitations. This documentation
review does not establish scrolling, performance, physical input or VoiceOver results.

Read internal `t`, `text`, `initial`, the accessors/bounds, `with_streamed_lines`,
then `apply`. The abstract model stores a typed
`(int,string,Int.comparator_witness) List_collection.t`, first/last keys and latest
extra-line count. `List_collection.of_alist (module Int)` uses an explicit typed
comparator and validates unique keys/order. Initial history contains keys 0–999,
with first=0, last=999 and streamed_lines=0.

`text id` formats Entry N plus one to three repeated base lines according to
`1 + abs(id) % 3`. Negative keys represent prepended history; key magnitude is an
identity/fixture convention, not an inferred pixel position. The finite [-32,1031]
domain makes integer arithmetic and `abs` safe here. Immutable data values do not
allocate native rows or determine which rows are currently mounted.

## Actions preserve identities and old response text

Append allows last up to 1031: it splices a new `(last+1,text)` at collection end,
updates last and resets the **new latest row's** extra-line counter to zero. It does
not erase fragments previously accumulated in the older latest row. Prepend allows
first down to -32, splicing a new stable negative key at index zero without changing
last or streamed_lines. At their respective bounds both actions return the same
model unchanged. With 32 additions at each end, history is bounded to 1,064 rows.

Stream_latest calls `with_streamed_lines` with current count+1 capped at eight.
That helper rebuilds the latest value from `text last` plus that many “Another useful
detail.” lines, then uses `List_collection.set ~key:last ~data`. Only the value changes;
source identity and key order stay intact. Reset_latest requests zero extra lines.
When the count already matches, the helper returns the model unchanged, so repeated
capped growth/reset avoids another allocation. Raising `ok_exn` unwraps checked
collection operations; the public API validates keys and splice positions in
[list_collection.mli](../../../lib/core/list_collection.mli).

## Trace latest growth and a new message through Bonsai

The caller constructs `B.state_machine0 ~default_model:Messages.initial`, using
`Messages.apply` as the reducer. A state machine holds a reactive model and an
injection effect; this module itself only maps ordinary model/action values.
`B.map messages ~f:Messages.rows` feeds the managed list, which uses Int keys and
`Key.of_int` for row identity. Native code measures rows and owns scrolling;
Bonsai's `let%arr` reads active row text/palette and derives its GPUIO view.

Click **Grow latest response** twice: key 999 receives two extra lines while all
row keys stay the same. The native list updates that row's height/content and can
follow tail when already at the end. Click **New message**: Append adds key 1000,
resets streamed_lines to zero for that new latest response, and preserves key 999's
two extra lines. Click **Earlier history**: Prepend adds -1 ahead of the original
history; stable existing keys let native anchoring preserve the entry being read.
Growth now affects key 1000; Reset latest removes only that row's extra lines.

The page's list has estimated 85-pixel rows, 235-pixel viewport, max_active=24 and
100-pixel overscan. Those native/managed computation budgets are distinct from this
model's 1,064-row data bound. Accessibility uses a named Log with Live.Off, so growth
is not automatically announced. [Message_follow](message_follow.md) derives the
follow-button overlay from viewport observations and sends explicit jump commands;
this model neither scrolls automatically nor allocates that controller.

The model persists in its owning Bonsai graph; native mounted row lifetimes are
independent. No file/network producer requires cancellation here. Leaving/remounting
managed rows does not imply deleting model history; controller teardown and stale
scroll fences belong to the caller/runtime. The reducer needs no dispose function.

[Existing expect tests](../../../test/gallery/message_stream_test.ml) check value-only
changes for streaming, key-order sharing, old streamed content after append/prepend,
reset and repeated actions reaching (-32,1031,1064 rows,8 lines). They were read as
existing behavior tests, not freshly executed or approved as extra source groups.

To use a larger finite history, change initial_count/extra_limit and revisit bounds
and native data budgets, rather than relaxing only the visible button's disabled
state. For a real async response, give requests their own typed identity and cancellation
policy outside this pure model; target the response key explicitly so a late fragment
cannot accidentally append to whichever row has since become latest. Preserve stable
keys and use value replacement for content growth instead of remounting a row under
a new identity for every fragment.
