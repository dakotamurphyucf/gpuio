# Diff state: choose native-managed or application-controlled expansion

[diff_state.ml](diff_state.ml) and its [interface](diff_state.mli) define a pure
reducer for the gallery's [Documents page](../documents_page.md). They translate
application choices and queued native diff observations into `Document.Diff.Config`
and a notice. The module performs no parsing, file access, Bonsai graph construction,
Eio work or native resource allocation.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Markdown & code → Diff**. Collapse a file, use Show more, toggle
**Application controls expansion** or **Emphasize changed words**, then append/reset
a file. All patches are mock strings; parsed file paths are labels, not files read
from disk. This model has no independent executable or diagnostic flag. Runtime
prerequisites and platform scope are in the [page walkthrough](../documents_page.md);
this review claims no additional native or platform validation.

Read the internal `t`, `initial`, `config`, `observe`, then `apply`. The public model
is abstract. Internally it stores a controlled Boolean, collapsed typed file keys,
an optional row limit, word-diff preference and notice. `initial` is native-managed,
with no collapsed keys, a four-row limit, word diff true and the prompt to choose a
file or changed line. `None` for limit means unlimited, not zero. File identity is
`Diff.File_key.Path name` or `Unnamed`, distinct from snapshot-local file indices.
The [public diff interface](../../../lib/core/document_diff.mli) defines those keys,
row counts and source-provenance observations.

`config` constructs a validated `Diff.Config.t`. In managed mode it always sends
`Managed { initially_collapsed = [] }` and `Managed { initial = Some 4; step = 4 }`.
These are stable initialization seeds; re-deriving the view does not copy an observed
native collapse or line count back as a fresh instruction. In controlled mode it
sends `Controlled t.collapsed` and `Controlled t.limit`, making the model authoritative.
`word_diff` is applied in both cases. The constructor is unwrapped with `ok_exn`
because this reducer produces bounded, checked configuration.

## Reducer actions and ownership guards

`Action.t` has Toggle_controlled, Toggle_words, Reset and Observe of a public native
event. `apply` returns a new model rather than mutating its input. Toggle_controlled
starts from `initial`, flips ownership and preserves the word-diff preference.
Toggle_words changes only that preference. Reset clears collapse/limit/notice while
preserving both ownership and word preference.

`observe` treats each event according to current ownership:

- `Toggle_file`: native-managed observations require `applied=true`; controlled
  requests require `applied=false`. The equality guard rejects the opposite mode's
  delivery. Controlled duplicate requests matching the current collapsed membership
  also do nothing. A valid request filters out the old key, optionally reinserts it,
  and updates membership only when controlled; managed mode updates just the notice.
- `Show_more`: managed mode uses `applied_limit` only to report the native result,
  retaining stable seeds. Controlled mode requires `applied_limit=None` and a current
  finite limit equal to the event's visible count, then adds `min 4 hidden`, capped
  at 8192. Incompatible counts, applied echoes or an unlimited model do nothing.
- `Line`: the notice displays the typed file label, old/new one-based line numbers
  (an em dash for absent sides) and original payload text. It does not navigate to
  or edit a file.

These guards prevent feedback loops and specific delayed intents after ownership
changes; they are not a general source-revision validation mechanism. The reducer
does not compare the event's source revision/generation. A different application
must check provenance before applying snapshot offsets to new content. Here the
model uses durable file keys and counts rather than writing into source offsets.

## Follow a controlled Show more request

The caller [documents_page.ml](../documents_page.ml) constructs
`B.state_machine0 ~default_model:Diff_state.initial`, applying actions with
`Diff_state.apply`. This is Bonsai's state plus an injection effect; the pure module
itself is unaware of the graph. A switch runs `inject_diff Toggle_controlled`.
The page's `let%arr` reads the resulting model and supplies `Diff_state.config` to
`Document.Config.create ~diff` and then `V.document`.

Suppose controlled mode has limit four and the installed diff has one remaining
hidden row. Clicking Show more emits `Show_more { visible=4; hidden=1;
applied_limit=None }`. Native code reports intent instead of changing an authoritative
limit itself. The page's `on_diff` injects Observe; the reducer sets limit five and
notice “Showing up to 5 changed and context lines”. Bonsai derives Controlled (Some 5)
and GPUIO submits it, allowing native layout to display that fifth row. In managed
mode a comparable event instead reports `applied_limit=Some 5`; the notice changes,
but the model still derives Managed initial-four/step-four seeds. Native-managed
scroll/selection/expansion stay native without an OCaml callback during painting.

`Resources.append_diff` owns bounded source updates in the page; it is separate
from this expansion model. Reset diff requests the original document then injects
Reset on local success. Page deactivation also injects Reset; native source cleanup
belongs to `Preview_scope`, not to these pure values. There is no I/O cancellation
or resource disposal inside this module.

The [existing expect tests](../../../test/gallery/diff_state_test.ml) cover stable
managed seeds, controlled file intent, limit four→five, word toggle/reset and delayed
controlled intent after a switch back to managed ownership. They decode fixture
wire events through the public Expert adapter before reducing them; no new tests
were run for this guide. The [model Dune library](dune) uses Core, GPUIO and Jane
Street PPX; test registration is in [test/gallery/dune](../../../test/gallery/dune).

To use an eight-row increment, change both the Managed step and controlled
`Int.min 4 hidden` expression, preserving the 8192 cap and matching-visible-count
guard. If the initial limit changes, update both `initial.limit` and Managed's
initial seed. Keep stable keys and native ownership distinctions; copying observed
managed state into configuration on each event would turn observations into
unintended control commands.
