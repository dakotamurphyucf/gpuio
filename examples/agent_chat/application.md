# Assemble the Agent Workspace application

[application.ml](application.ml) turns shared conversations and independent
workspace components into a native multi-window application. Start with its
small [interface](application.mli), then `Application.run`, `read_file`,
`open_window` and the first `open_window 1`. The [CLI companion](main.md)
explains launch switches; the [fake backend companion](model/fake_backend.md)
explains the local response data. No account or remote service is needed.

From the repository root, use the [isolated environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

The native target is macOS 14.4+; Linux needs the display/driver prerequisites in
the development guide, and graphical qualification remains informational under
[the release policy](../../docs/platform-release-policy.md). The build links the
example counter's generated native backend through `gpuio_counter_backend` in
[dune](dune), because the review inspector consumes that component. This is
build-time composition; no dynamic plugin download occurs.

## Capabilities and ownership

`Gpuio_eio.App.run ~motion (fun env app -> ...)` supplies an Eio environment and
application controller. `env` grants capabilities: the clock, filesystem,
stdout and domain manager. `app` supplies the application scope and creates
windows. The module passes narrow capabilities to components instead of letting
those components fetch a global environment. For example, `sleep` is
`Eio.Time.sleep (Eio.Stdenv.clock env)`; it can suspend an Eio fiber without
blocking the UI thread in a system sleep.

`Conversation.create` makes three conversations numbered 1–3 under `App.scope
app`. Their titles and 200-row synthetic histories are fixtures, not persistent
storage. Their response documents can outlive an individual workspace view.
`Icons.create ()` prepares shared icon state; later `Icons.initialize icons app`
registers resources once. [conversation.ml](runtime/conversation.ml) and
[icons.ml](runtime/icons.ml) contain those implementations and their adjacent
interfaces define their contracts.

`open_window selected` first removes closed windows from the local `windows`
list. If fewer than four remain, it makes a fresh `Workspace.create` value and
opens an 1180 × 820 logical-pixel window through `App.open_window`. A serial
counter distinguishes titles; `selected` chooses a conversation ID rather than
an index into tabs. Each workspace gets its own editor drafts, retained tabs,
viewport and inspector state while receiving the same conversation objects.
See [workspace.mli](runtime/workspace.mli) and
[workspace.ml](runtime/workspace.ml) for the window component boundary.

Inside the window callback, `Sources.create` and `Results.create` receive
`App.Window.scope window`. Their async loaders therefore die with that window.
Large fixture construction delegates pure CPU work to `Eio.Domain_manager.run`:
`Source_data.large` makes synthetic nodes, and `Result_data.replace` prepares a
complete result query. These worker domains calculate data; they do not own
native views or run Bonsai effects. The fixture controllers in
[sources.ml](runtime/sources.ml), [results.ml](runtime/results.ml) and
[fixture_job.ml](runtime/fixture_job.ml) own bounded producer scheduling and
stale delivery; `Application.run` just supplies the capability. A cancellation
can retire delivery while an already-running CPU calculation drains.

`read_file` uses `Eio.Path.with_open_in` under the supplied filesystem capability
and closes the flow when the callback finishes. `Eio.Buf_read.take_all` with
`max_size:65537` provides the bounded reader needed to detect the demo's 64 KiB
attachment limit. The optional attachment directory is a picker hint, not a
restriction on readable paths. [Workspace.attach](runtime/workspace.ml) performs
picker selection and schedules reading in the originating window scope;
[Conversation.attach](runtime/conversation.ml) validates/adopts the result.
There is no file write or attachment persistence here.

## Graph construction versus native startup

`Workspace.component ... window` returns the Bonsai component passed to
`App.open_window`. A Bonsai graph describes dependencies between state and
rendered values; this startup module supplies its resources, while the workspace
constructs the graph. Reactive `let%arr` expressions and controls live in that
component, not in this file. GPUIO views describe native presentation, while Eio
scopes describe asynchronous work ownership; neither is a substitute for the
other.

`App.Window.on_change` runs a callback that returns a `Bonsai.Effect.t`. An effect
is a deferred action, rather than the window's view. The `resources_started` guard
makes that callback return `Effect.Ignore` after the first observed change. On
the first change, `Effect.Many` combines icon initialization and
`Conversation.initialize` for all conversations. Those effects create native
resources and publish the seeded Markdown/code/diff documents. The flag is
set before starting them, so this is one-shot initialization rather than an
automatic retry loop.

`Workspace.install_close_handler` installs the draft-aware window close policy.
The application keeps each `(window, workspace)` pair for later opens and tests.
`App.on_reopen` defers `open_window 1` using `Effect.of_thunk`, allowing the native
application reopen event to make a fresh workspace. `App.run` owns final runtime
cleanup; the scoped controllers retire their resources as their scopes close.
There is no disk-backed session and no promise of restoring drafts after exit.

## Trace another window and a send

1. **New window** reaches the workspace command and calls the supplied
   `open_window` function with the current conversation ID. It creates another
   workspace and window scope but passes the existing conversation list.
2. A native composer submit creates an `Input.Submission`; workspace `submit`
   captures the prompt/configuration and asks `Conversation.submit` to accept it.
   The acceptance task belongs to this window until accepted.
3. Accepted response production belongs to the conversation scope. Its bytes
   update the shared response document and its phase updates Bonsai observations
   in both workspaces. Each window still has its own native editor and viewport.
4. Closing the originating window during acceptance cancels that pending send.
   Closing it after acceptance retires its editors/source/result loaders while
   the conversation stream can continue in another window. Closing the app
   cancels the application scope and shared resources too.

The [backend trace](model/fake_backend.md#follow-a-send-into-the-runtime) follows
chunk delivery, completion/cancellation and conditional draft clearing in more
detail. Native command submission or document publication does not prove a frame
was physically presented; [existing evidence](../../docs/evidence/agent-workspace-m4.md)
qualifies its own recorded source/platform checks.

## Optional runners and adaptation

Ordinary launch passes `self_test`, `native_test` and `workload_metrics` as false.
`--self-test` starts [Self_test.start](self_test.ml) and asserts its pass flag after
`App.run` returns. It opens real windows and drives controllers; it is not an
external keyboard/IME test. `--native-test` makes send acceptance last one second
and prints `GPUIO_AGENT_CHAT_NATIVE_APP_RETURNED` after shutdown, helping the
external scripts observe the newer-draft race and process completion.
`--workload-metrics` selects seven-byte/five-second streaming and starts
[Workload_metrics.start](workload_metrics.ml), an app-scoped half-second stdout
sampler. Its task contributes to the diagnostic counts it prints. The native
scripts and their permissions are listed in the [README validation map](README.md#validation).
These supporting runners remain optional verification code rather than a
starting application template.

A small adaptation is to rename or add a seeded conversation in the title list.
`List.mapi` assigns IDs in order, and `open_window 1` assumes ID 1 exists, so keep
the list nonempty and account for the window's retained-panel bound if expanding
it. For a new local loader, pass its capability here and bind its controller to
the appropriate application or window scope; do not infer its lifetime from
whether its page is currently visible. If replacing the file reader, preserve
bounded input, flow closure and the originating-window cancellation boundary.
The [application contract](../../lib/eio/app.mli),
[scope contract](../../lib/eio/scope.mli) and
[ownership design](../../docs/design/agent-workspace.md) are the relevant public
and application references.
