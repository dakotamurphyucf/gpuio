# Deterministic response model

The chat demo needs a reproducible response so you can try streaming, interruption
and retry without a provider account. This module supplies response bytes and
three initial artifact fixtures. It is ordinary application logic: it has no
Bonsai graph, GPUIO handles, Eio tasks, filesystem access or actual waiting.
The visible streaming behavior comes from the runtime consuming its plan.

From the repository root, after the isolated [environment setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open **Demo controls**, select **Simulate error**, send a prompt, then choose
**Retry response**. A normal retry clears that response document and streams the
same answer again. The ordinary launch needs no network or external fixture
files. macOS is the v1 desktop target; required Linux build/unit checks and
informational Linux GUI checks have [separate scope](../../../docs/platform-release-policy.md).

## Read the model

Read [fake_backend.mli](fake_backend.mli) for the contract, then
[fake_backend.ml](fake_backend.ml) for `Config`, `Step`, `response` and `plan`.
[model/dune](dune) builds `gpuio_agent_chat_model` using Core and `ppx_jane`;
it does not link Bonsai or the native backend. Callers use the qualified name
`Gpuio_agent_chat_model.Fake_backend`.

`Config.t` is abstract outside the module. `Config.create ... ()` returns
`Or_error.t`, so invalid settings produce an error instead of an unchecked
record. The final `()` lets OCaml finish applying the optional arguments and use
their defaults. The accepted settings are:

| Field | Default | Invariant |
| --- | --- | --- |
| `chunk_bytes` | 17 | Integer from 1 through 4096 |
| `delay_seconds` | 0.02 | Finite seconds from 0 through 10 |
| `accept_delay_seconds` | 0.25 | Finite seconds from 0 through 10 |
| `fail_after_chunks` | Absent | If present, integer from 0 through 65536 |

`Config.delay_seconds config` and `Config.accept_delay_seconds config` expose
only timing metadata; neither accessor sleeps. `[@@deriving equal, sexp_of]`
generates typed equality and printable S-expressions for comparisons/diagnostics,
without adding a way to construct an invalid configuration.

`Step.t` is an exposed variant: `Chunk of string`, `Fail of string`, or `Finish`.
Pattern matching lets the runtime handle each case explicitly. A chunk contains
**bytes**, not a promised complete Unicode character. OCaml's `String.length`
and `String.sub` count bytes here. The UTF-8 family emoji and lambda in the answer
are intentional stress data for the document decoder.

`response ~prompt` formats fixed Markdown including the prompt's byte count;
it does not echo the prompt, parse it, infer an answer or validate it. Prompt
validation belongs to `Conversation.submit`. `plan config ~prompt` computes
`count = (text_bytes + chunk_bytes - 1) / chunk_bytes`, the number of chunks needed
including the shorter final chunk. It emits up to `fail_after_chunks` chunks,
then exactly one terminal step. If the limit cuts the response short it emits
`Fail`; if the limit equals or exceeds the total it emits `Finish`. A limit of
zero therefore produces an immediate failure without any chunk. The plan is a
fully allocated finite list, appropriate for this short fixture rather than a
live unbounded provider stream.

`markdown_fixture`, `code_fixture` and `diff_fixture` are literal sample strings.
`Conversation.initialize` installs them in rows 198–200 as Markdown, OCaml code
and a diff. The diff is displayed, never applied to a file; the code is displayed,
never executed. `workspace.inspect` is a sample author label, not a tool call.

## Follow a send into the runtime

Read [application startup](../application.md), then these consumers:

- [workspace.ml](../runtime/workspace.ml): `send` asks the native editor to submit;
  `submit` receives an `Input.Submission`, captures the window's current backend
  configuration and passes its text to `Conversation.submit`.
- [conversation.ml](../runtime/conversation.ml) and
  [conversation.mli](../runtime/conversation.mli): `submit`, `accept_response`,
  `start_stream`, `finish`, `cancel` and `retry` own the asynchronous work.

For example, sending `λ` with `chunk_bytes = 1` records a two-byte prompt.
`Conversation.submit` enters `Accepting` and starts a window-scoped task whose
Eio sleep uses `accept_delay_seconds`. After document creation succeeds,
`accept_response` appends user/assistant rows and moves production to a child of
the shared conversation scope. The accepted callback calls
`Editor.clear_if_unchanged`, which clears the captured submission only if the
native editor has not acquired a newer draft.

`start_stream` sets `Streaming`, calls `plan`, and sleeps before each `Chunk`.
It checks the active response token/scope before calling
`Gpuio_eio.Document.push_bytes`. The [document contract](../../../lib/eio/document.mli)
buffers up to three trailing bytes when a scalar is split; callers must not
replace this with an API that demands each chunk be standalone UTF-8.
On `Finish`, `finish` calls `Document.finish` and sets `Complete`. On `Fail`, it
calls `Document.cancel`, retains completed Unicode content and records `Failed`.
The `Fail` message is explanatory text, not a structured provider error type.

Bonsai makes changed application data observable: `phase_value` supplies a
reactive value (a value whose dependents update when it changes), while
`Pager.value` supplies transcript rows. In `workspace.conversation_panel`,
`let%arr ... and ... in` combines their current values into the view; it is
Bonsai syntax for deriving a value, rather than reading a fixed initial snapshot.
`List_view.paged` gives rows stable integer keys and bounds mounted rows;
`message_view` renders the document through GPUIO. An `Effect.t` describes an
action to run later, such as submitting a native command or updating a phase;
it is distinct from a derived view. None of this reactive syntax belongs in the
pure backend itself.

Cancel clears the active producer and cancels its scope; an incomplete trailing
scalar is dropped while completed text remains. `current` rejects late results
using both token identity and active scopes. Retry resets the same response
document, obtains a new token and reruns a plan with the retry configuration.
Hiding a row or closing a tab does not own the producer lifetime. A pending send
belongs to its originating window; an accepted response belongs to its shared
conversation, and other windows can continue observing it. These boundaries are
explained in the [ownership design](../../../docs/design/agent-workspace.md).
A successful append/publication is asynchronous native acceptance, not proof of
parsing completion or an actual displayed frame.

## Check and adapt the fixture

[fake_backend_test.ml](fake_backend_test.ml) is the supporting inline expect test;
it shares this guide because it checks this module's contract. Run its focused
nongraphical check with the development profile:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/agent_chat/model -j 2
```

`let%expect_test` registers a test through `ppx_jane`. The test concatenates chunks
for sizes 1, 3, 17 and 4096 and compares them with `response` using `String.equal`.
It checks that the last step is `Finish`, prints the three-chunk failure plan,
and checks rejection of zero chunk size, NaN delay and negative failure count.
`[%expect ...]` records the expected S-expression output. These checks do not
measure delays, prove decoder behavior or validate physical GUI input. The
runtime's guards, decoder and native presentation require their own checks.

A small adaptation is to add another Markdown section in `response` and change
`code_fixture` to the sample you want readers to inspect. Keep the full fixture
valid UTF-8, keep byte-based slicing, and update the corresponding expect output
if the first chunks change. To make slow typing easy to observe, construct a
configuration with `Config.create ~chunk_bytes:7 ~delay_seconds:0.06 ()` in the
workspace's demo preset; handle constructor errors if settings come from users.
Running streams already capture their configuration, so changing the preset
affects later sends. Replacing the fixture with a real provider requires an Eio
service in the runtime, typed recovery errors and the same scoped cancellation/
stale-result discipline; putting network I/O into `plan` would erase the useful
pure-model boundary.
