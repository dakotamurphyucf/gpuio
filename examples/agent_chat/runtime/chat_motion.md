# Native motion around workspace content

[chat_motion.ml](chat_motion.ml) and [chat_motion.mli](chat_motion.mli) build three
small `Gpuio.Animation.Program` descriptions: a stage-context spring, ordered
workspace destination reveals and shared response activity. They return
`Gpuio_bonsai.View.t` wrappers. They own no Bonsai model, Eio sleep or timer task:
native presentation evaluates the configured motion between OCaml updates.

Build from the repository root using the
[isolated environment](../../../docs/development.md), then compare explicit motion policies:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe --full-motion
_build/default/examples/agent_chat/main.exe --reduced-motion
```

Run the launches separately, closing the first before starting the next. Open
**Explore workspace**, then **Explore run diagram**, open a stage, and toggle
**Show stage context** twice during the reveal. Send a prompt to see the header
and composer activity. Both policies leave the same working controls available;
the override affects this application without changing OS settings. macOS is the
v1 native target; Linux GUI qualification remains
[separate/informational](../../../docs/platform-release-policy.md). No external
assets or remote data are needed for this simulated run.

## Build validated programs

Read the helper constructors first: `target` wraps `Animation.Target.create`,
`stage` wraps `Animation.Stage.create`, and `tween` uses `Timing.tween` with
`Easing.ease_out`. `ok = Or_error.ok_exn` is appropriate for the checked-in literal
configuration; user-supplied parameters should handle validation errors instead.
Targets name numeric properties, timings explain their interpolation and stages
combine the two. `Animation.Program.create` combines stages into a retained
native program. The [animation interface](../../../lib/core/animation.mli)
specifies valid ranges, repeated-program constraints and reduced-motion policy.

`View.animate_program ~key ... program children` puts the program around ordinary
views. Its stable key identifies the retained animated wrapper in its view
location, rather than a globally unique animation. New target bodies retarget
from painted values; unchanged descriptions can survive ordinary view rebuilds.
No helper installs `~on_event`, so completion/cancellation notifications do not
change application state here. The [view contract](../../../lib/core/view.mli)
explains optional ordered endpoint observations and native disposal.

| Helper | Program | Caller-owned data |
| --- | --- | --- |
| `stage_context ~expanded children` | One spring stage targeting Height 112 or 0 | Expanded Boolean and content |
| `destination ~index content` | Opacity 0.35 → 0.72 over 100 ms → 1 over 170 ms | Destination order and action |
| `activity ~key ~group content` | Opacity 1 → 0.35 over 650 ms, alternating repeat | Busy status, identity and group name |

The context spring has stiffness 210, damping 27, mass 1, tolerance 0.05 and a
1.5-second maximum duration. Geometry is in logical pixels. Its single stage has
no explicit initial target: the first mount establishes its target; subsequent
target changes animate. The outer `stage-context-motion` wrapper clips vertical
overflow and does not shrink. Its inner `View.panel` has height 112,
`~active:expanded` and `~hidden:Unmount`. Closing immediately removes the inner
native controls/semantics while the wrapper height can settle toward zero; it
does not retain interactive children for the closing animation. Panel hiding
does not destroy the caller's Bonsai state.

`destination` clamps the delay calculation to indices 0–8, multiplying by 55 ms.
The stable key still uses the original index, so distinct indices remain distinct
identities even when their delays clamp to the same value. Its two stages require
an explicit initial opacity. The actual overview uses indices 0–5, so it creates
six ordered reveals with delays 0–275 ms. It does not schedule navigation or
change what a destination button does.

`activity` uses `Animation.Clock.group group` and `~repeat:Alternate`. Group
names are validated application-scoped UTF-8 strings, 1–128 bytes without NUL.
Shared clocks require repeating timed programs with explicit initial values and
zero initial delay; this helper meets those constraints. The same group can align
members in independent windows. Reduced motion holds this legacy repeated
program at its initial opacity and requests no repeating frames; finite context/
destination programs settle at their endpoints. Hidden independent animations
pause; shared members rejoin the current group phase when shown.

## Trace the owning state and lifetime

In [inspector.ml](inspector.ml), `component` creates a
`Bonsai.Cont.state_machine0` with initial `context_expanded = false`. Its action is
`unit`; `apply_action` toggles the Boolean. This constructs a dependency graph
node and returns a reactive model plus an action-injection function. The
`let%arr ... and context_expanded = context_expanded and toggle_context =
toggle_context in ...` expression derives the inspector view from their current
values. A native **Show stage context** activation runs the injected Bonsai
effect, toggles the model, changes the button label and passes `expanded = true`
to `stage_context`. The wrapper's key stays constant, so a second activation
changes the target back to zero and native motion retargets mid-flight.
The context says that execution is simulated and sources are unchanged; no tool
run or file I/O is initiated. The Boolean belongs to this window's inspector
component, rather than a separate state per stage visit.

The overview branch passes already-created destination buttons to `destination`.
Their click effects still call inspector `navigate`; opacity does not encode a
route or store history. Leaving the overview removes those wrappers normally.

In [workspace.ml](workspace.ml), `conversation_panel` derives `busy` from the
observed `Conversation.phase_value`: `Accepting | Streaming` are busy; terminal
states are not. It mounts `activity` only in busy branches, using distinct keys
`header-activity` and `composer-activity` and the common group
`chat-response-<conversation-id>`. The header wraps the loading spinner; the
composer wraps its status dot. Both use native phase while the same conversation
can appear in multiple windows. On completion or cancel the phase changes,
Bonsai derives the idle branch and removes the animation wrappers. Thus an idle
conversation has no need for an OCaml tick loop or mounted response pulse.
The [backend trace](../model/fake_backend.md) explains Eio work cancellation;
removing these decorative wrappers does not cancel that producer or release its
document. Window disposal releases mounted native animation state separately.
No individual native frame reruns the Bonsai graph.

## Adapt without changing ownership

For a slower reveal, adjust `tween 100.`/`tween 170.` or the 55 ms ordering delay;
keep the explicit initial target and finite duration bounds. If context height
changes, update both the spring target and the inner panel's fixed height: the
112-pixel choice currently assumes the supplied four short text rows fit.
Keep `stage-context-motion` stable when toggling; generating a fresh key on each
update would prevent retained retargeting. For a second simultaneous context
wrapper under the same native parent, give it a distinct sibling key rather than
reusing this helper's fixed key unchanged. Activity members that should share
phase need the same group; unrelated workloads should use distinct names.

The optional `python3 scripts/test_agent_chat_motion.py` runner exercises actual
macOS geometry/interruption and streaming input; prerequisites and recorded
coverage are in the [README](../README.md) and
[milestone evidence](../../../docs/evidence/agent-chat-m5.md). This documentation
review does not rerun that test or establish painted-motion/keyboard acceptance.
A submitted animation target is also not an observation of a displayed frame.
