# Observe a separately packaged native review counter

[review.ml](review.ml) and [review.mli](review.mli) put the example native counter
inside Agent Workspace's review inspector. The counter handles pointer/keyboard
activation natively; Bonsai observes its value and supplies properties/explicit
reset commands. This is an application **consuming** an extension package, not a
Rust extension-author implementation or a simulated OCaml-only button counter.

Use the [isolated environment](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Explore workspace/Review checkpoints, click the checkpoint card or focus it
and press Space/Enter, choose +1/+5, then Reset review. The application links the
counter through its generated native backend in [agent_chat/dune](../dune);
no dynamic plugin download or network connection occurs. macOS is the v1 target;
Linux GUI checks remain [informational](../../../docs/platform-release-policy.md).

## Read the graph and typed package boundary

`component ~dark graph` builds five `Bonsai.Cont.state` nodes: observed value 0,
step 1, no pending command, serial 0L and Ready to review status. Each state call
returns a reactive value plus a setter effect function. Reactive values update
dependent view derivations; `let%arr ... and ... in` combines the current values
and setters to build presentation. Setter calls return deferred Bonsai effects,
not mutations performed while building the graph.

The package's [public interface](../../extension_package/ocaml/gpuio_example_counter.mli)
defines `Counter.Properties.create ~value ~step ()` and `Counter.instance`.
Values are 0–100, steps 1–10. This module offers only 1 and 5 choices. It passes
observed value/selected step as validated properties, stable generation 1L and
an optional `set_value:(positive sequence, new value)` command. Incrementing
generation would explicitly reset native state; ordinary view updates here
keep generation fixed so properties/remount seeding and native interaction have
stable meaning. Read the [extension-package walkthrough](../../extension_package/README.md)
for the adapter's build/link/command contract.

`View.extension ~key:"review-counter" ~on_event instance` mounts that typed
instance. Its events are handled explicitly:

| Event | Bonsai effects |
| --- | --- |
| `Data count` | Store the observed count; status Review updated |
| `Command_completed sequence` | Clear pending command/status Review reset only if sequence matches |
| `Mounted` | Ignore |
| `Failed _` | Status Review component unavailable |

The failure branch intentionally shows a generic status; it does not clear the
pending command or present a retry policy. No Eio task or filesystem operation
belongs to this component. Native events arrive through GPUIO's queued extension
boundary, not synchronous OCaml callbacks during paint.

## Trace activation and acknowledged reset

A native counter activation advances its state according to current step and
emits typed Data. GPUIO queues that event to the OCaml handler, whose effects
store the observed value and status. Bonsai derives updated properties and
Recorded progress text (N of 100 checkpoints); the native extension reconciles
that state through its package API. Choosing +5 changes the supplied property,
not a second independent counter value.

Reset review computes the next serial and returns `Effect.Many` containing
updates to serial, pending `(serial, 0)`, observed value 0 and Resetting review…
status. The local value therefore changes optimistically, while the status
waits for the native `Command_completed` sequence. A matching acknowledgement
clears the pending command and says Review reset; an unrelated sequence is
ignored. Submitting the command is distinct from native command completion or
a physically painted frame. Ordinary Data events have their own handler and
may update status to Review updated; this source does not promise an absolute
ordering of all native observations relative to command submission.

Presentation uses Palette.of_dark and Presentation.Appearance, a group box,
choice/reset buttons, stacked description entries and a locally saved marker.
The choice styles reflect the selected step, and focused secondary buttons use
an accent border. Labels describe session state: nothing is saved to disk or
sent as review telemetry.

## Retention and adaptation

[Inspector.component](inspector.ml) constructs Review.component outside its
route selection and keeps its graph alive across route changes/native inspector
unmounts. Hiding/removing its native page retires native counter content, while
Bonsai's observed count/step remain. Reopening seeds a new native mount from
those properties. Another window constructs a different graph and progress;
window closure ends the state. Stable view key, fixed instance generation and
explicit command sequence serve different identities and should not be merged.

A small adaptation is a +2 option: add `choice 2`, keeping package step limits
and the single state/value/event flow. For a different maximum, the package's
0–100 contract must change together with its native implementation/schema and
progress labels; changing only N of 100 text does not alter the counter's limit.
If displaying detailed failure information, pattern-match the typed extension
error rather than treating Failed as a count update. The public
[extension interface](../../../lib/core/extension.mli) explains the general
instance/event boundary.

`python3 scripts/test_agent_chat_review.py` is the optional macOS native runner;
[README](../README.md)/[recorded evidence](../../../docs/evidence/agent-chat-m5.md)
qualify properties, events, commands and lifetime checks. This source review
runs no native counter/keyboard test and adds no extension acceptance claim.
