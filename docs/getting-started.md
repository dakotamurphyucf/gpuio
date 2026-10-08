# Build an OCaml native application

GPUIO is experimental; there is no stable API release yet. Milestone 07 targets
macOS first, with macOS 14.4 as the current native deployment target and local
qualification on arm64. Linux remains experimental/build-tested; full desktop
qualification is deferred to OCH-47. See [platform policy](platform-release-policy.md)
and [current implementation status](status.md) before relying on a capability.

## Build the starter

Use the [development prerequisites](development.md), clone the repository and run
these commands from its root:

```sh
./scripts/gpuio bootstrap
GPUIO_JOBS=2 ./scripts/gpuio build examples/getting_started/main.exe
_build/default/examples/getting_started/main.exe
```

The last command opens an interactive window. Increment and Reset update Bonsai
state; Close requests the normal window-close decision. Closing the last window
ends this application. Builds alone do not open a window.

The complete [main.ml](../examples/getting_started/main.ml) and
[Dune stanza](../examples/getting_started/dune) are the example, rather than a
separate pseudocode listing. Use the checked-in Jane Street formatter and PPX.
`App.run` owns the OS main thread and starts the OCaml UI domain with Eio; call it
from the program entry point, without wrapping it in another `Eio_main.run`.
Construct Bonsai state once while building the component graph. Reactive view
recomputation returns immutable descriptions and typed effects.

## Choose the public layer

| Dune library | OCaml module | Use |
| --- | --- | --- |
| `gpuio` | `Gpuio` | Validated types, view/style descriptions, configuration and resource sources; no Bonsai/Eio scheduler |
| `gpuio.bonsai` | `Gpuio_bonsai` | Views whose callbacks carry Bonsai effects and reusable Bonsai composition |
| `gpuio.eio` | `Gpuio_eio` | Application/window runner, native controllers, scoped I/O and resource uploads |
| `bonsai` | `Bonsai.Cont` | Reactive application state and lifecycle |

Ordinary applications start with those libraries. `gpuio.native` selects the
statically linked backend through the runner dependency; native OCaml executables
are supported, not bytecode/toplevel loading. The default backend needs no Rust
application code. Component/document-profile packages add a generated backend as
described in [extensions](design/extensions.md) and
[the profile package](../examples/document_profile_package/README.md).
`Expert`, protocol and runtime-core interfaces support adapter authors; they are
not necessary for ordinary button, input or document composition.
The [API layer map](api-layers.md) identifies these integration boundaries and
the ownership/completion rules for common application operations.

## Ownership rules that affect application code

- OCaml owns application data and Bonsai state. Rust owns native layout, focus,
  editing, selection, animation and rendering. Effects and events cross the bridge
  asynchronously; a native acknowledgment does not prove physical presentation.
- Use stable `Gpuio.Key` identities for changing collections. A native remount is
  distinct from recomputing a description. Keep durable records/drafts outside
  virtual row computations; see [managed lists](design/managed-lists.md).
- Use `Gpuio_eio.Text_input` for editor observations and explicit commands. An
  observed value is not a controlled-value rewrite. Conditional edits use a source
  revision; geometry queries additionally carry an explicit snapshot. See
  [native editing](design/native-editor.md).
- Do file/network/timer work through Eio capabilities passed by `App.run` and
  the scoped task APIs. Window work belongs to its window scope; conversation or
  application work that must survive a view belongs to a longer-lived scope.
  Do not block Bonsai callbacks or start unmanaged background work during render.
  See [runtime](design/runtime.md).
- Native capability results distinguish unsupported, unavailable, canceled and
  successful work according to each operation's typed result. Check capabilities
  and handle errors; a Linux build does not imply every OS integration is present.

Use validated constructors for external values. `_exn` constructors suit known
literal constants. Styles use logical pixels and explicit units; they do not
promise browser CSS behavior. See [typed UI and inheritance](design/typed-ui.md).

## Consume installed public libraries

The following check stages public GPUIO and vendored Jane Street libraries under
a fresh prefix, copies the starter into a separate Dune project, and links it
against the installed default native backend:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py \
  --example getting_started --workspace scratch/starter-consumer-001
```

Choose a new workspace path on each run. The command does not open a window or
install anything into an opam switch. Its output records the workspace and
installed prefix; inspect `consumer/dune-project`, `consumer/dune`, and
`consumer/_build/default/main.exe` there. The script's historical name also covers
component consumers. `--run` is intentionally unavailable for this interactive
starter; launch its binary yourself when a desktop walkthrough is wanted.

For your own separate project, use the same native toolchain, installed packages
and `OCAMLPATH=<prefix>/lib` setup demonstrated by that check. Copy the starter's
Dune dependencies and PPX stanza and the repository's formatter/toolchain pins.
The installed libraries and native archive must come from the same GPUIO checkout.
Staging an isolated prefix is the current verified development path, not a claim
that an upstream opam release or standalone binary SDK has been published.

## Grow the application

Start with [Component Studio](../examples/gallery/README.md) for public component
examples, [Agent Workspace](../examples/agent_chat/README.md) for a streaming chat
application, and [Signal Studio](../examples/signal_studio/README.md) for graphics.
Use the [catalog](catalog/README.md) for exact functional mappings and remaining
acceptance, rather than treating a screenshot as proof of a whole component family.

Read [API compatibility and limits](api-compatibility.md) before persisting data
or combining packages, and [distribution](distribution.md) before shipping an app.
The [developer preview guide](developer-preview.md) explains the adoption scope
and how to report bugs, missing capabilities or confusing APIs with a small example.
