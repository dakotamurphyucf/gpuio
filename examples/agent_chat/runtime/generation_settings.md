# Accepted parameters for later simulated sends

[generation_settings.ml](generation_settings.ml) and
[generation_settings.mli](generation_settings.mli) define a small immutable
settings value: stream chunk **bytes** and interval **milliseconds**. This pure
module has no Bonsai graph, native control, Eio timer or provider configuration.
It translates accepted application settings into the
[fake backend configuration](../model/fake_backend.md).

Build/run from the repository root using [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Settings → Generation, change Stream chunk size/Stream interval, commit,
then send another prompt. No network or provider account is involved. macOS is
the v1 target; Linux graphical coverage is
[informational](../../../docs/platform-release-policy.md).

## Model and validated updates

`t` is an abstract record behind the interface, with derived typed equality and
S-expression printing. `default` is 17 bytes and 20 ms. Accessors return those
accepted values. `with_chunk_bytes t value` returns an updated immutable value
only for integers 4–128; `with_interval_ms` accepts 10–200 ms in ten-millisecond
steps. Invalid updates return `Or_error`, leaving the old value usable.
These narrower user-facing ranges are deliberate subsets of backend limits.

`backend t` calls `Conversation.Backend.Config.create` with the accepted byte
count and `Float.of_int interval_ms /. 1000.` seconds. It retains the backend's
default acceptance delay/no forced failure. `Or_error.ok_exn` relies on this
module's abstract validated value invariant; changing ranges requires reviewing
that translation. Chunks are not tokens, glyphs or complete UTF-8 scalars, and
interval is a simulated delay rather than a throughput guarantee.

## Where Bonsai and native editors enter

Read `generation_page` and `accept_generation` in [settings.ml](settings.ml).
The settings controller stores `G.default` in its window-owned observable model.
Native `View.number_input` has a Numeric.Domain of 4–128 with step 1; its initial
committed number comes from `chunk_bytes`. Native editor drafts/composition/
validation are separate from accepted `G.t`. Only a Committed snapshot with a
Number calls `with_chunk_bytes`; Empty reports an error. Changed/Observed/
Rejected update error presentation, and Cancelled clears it rather than saving
a draft. Native step controls alter the same numeric editor.

`View.slider` for interval uses a domain 10–200, step 10. Drag_started/Preview
change the settings model's temporary `interval_preview`, not `G.t`. Committed
calls `with_interval_ms` and clears that preview; Cancelled discards preview.
The settings model's `let%arr` view derivation reacts to saved/preview values.
Its effects execute deferred model updates, fenced by an outer settings epoch,
so retired page callbacks cannot save into a new settings visit. Closing/changing
pages clears ephemeral previews and native draft owners, while accepted settings
remain in the window model.

`accept_generation` invokes the supplied `on_generation` callback and stores the
new `G.t`/notice. [Workspace.component](workspace.ml) supplies a callback that
calls `set_backend (Generation_settings.backend generation)`. On the next editor
submission, workspace passes the current backend to Conversation.submit, which
captures it for acceptance/stream production. Changing settings mid-stream does
not rewrite its already captured configuration. This is the interaction trace:
native commit → validated pure update → window model/backend update → later send
captures it → scoped Eio producer sleeps/pushes those byte chunks.

Demo controls' Normal/Slow/Simulate error presets are explicit backend overrides;
they can replace the workspace backend independently of the saved generation
settings. A subsequent settings commit translates saved generation parameters
again. Reset generation preferences uses the outer settings confirmation and
restores this module's default; it does not clear result score filters or create
a provider connection.

A small adaptation is a wider byte range: update this validator and native numeric
domain together, preserve the backend's 1–4096 limit and all unit labels, and
keep committed values distinct from drafts. For a new provider setting, define
its own validated application contract rather than interpreting this fixture's
byte pacing as provider token limits. Public
[number input](../../../lib/core/number_input.mli),
[slider](../../../lib/core/slider.mli) and
[document](../../../lib/eio/document.mli) contracts explain native drafts and
split-byte decoding. The optional settings runner/evidence is listed in the
[README](../README.md); this source review adds no native editing or timing
acceptance result.
