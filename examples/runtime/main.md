# How `main.ml` exercises the public runtime's scopes and clocks

[README](README.md) · [Source](main.ml) · [App contract](../../lib/eio/app.mli)

`B` means `Bonsai.Cont`, `E` means `Bonsai.Effect`, and `View` is the Bonsai view
adapter. Ordinary launch opens two windows; `--self-test` runs a scripted native
integration scenario. Other flags select focused shutdown/backtrace checks.

`component` installs local count/awake states and `B.Clock.sleep`. Its activation
effect logs activation, captures a timestamp, sleeps 50 ms, records lateness, and
sets awake. `B.Edge.lifecycle` owns activate/deactivate callbacks; `Edge.on_change`
logs when awake becomes true with typed equality. `let%arr` derives effects/views
from reactive values, whereas `E.Let_syntax`'s `let%bind` sequences effect results.
The `and` bindings observe dependencies rather than starting threads.

Click Count: a native event runs the setter effect, Bonsai updates local state,
and the view's button label changes. The shared stream is an external
`B.Expert.Var` observed by both graphs. Native focus/paint stays native. Close
window uses force-close through a thunk; this diagnostic is not an unsaved-work
close-decision example.

`App.run` owns the native OS thread and an Eio UI domain. It uses 60 Hz ordinarily
or 5 Hz in self-test. A window opened and immediately closed must never activate
its graph. Two 480 × 300 windows then mount. A child conversation scope owns a
capacity-32 stream and producer pushing 1–10 every 30 ms; `on_batch` sets shared
state to its final value. A separate first-window task waits for cancellation.
Conversation work survives that window; see [scope](../../lib/eio/scope.mli).

The self-test waits for both activation timers and stream value 10, checks no
activation of the discarded window, then verifies idle clock ticks increase
without view commits. It measures request-frame callback latency and delivery
latency after an Eio producer finishes. It closes the first window and checks
window-task cancellation while conversation/second window survive. With
`exit_on_last_window:false`, it closes both, opens/closes a third, and checks
lifecycle deactivations. Another graph closes itself during activation; its
deactivation must still arrive. Success prints `RUNTIME_PASS` then shuts down.

The initial readiness wait has a 10-second deadline, and frame/task/deactivation
promises have 3-second deadlines. Some later stages use fixed sleeps, so this is
not one global end-to-end timeout. Reported timer lateness/frame/task latency are
scenario observations, not universal scheduling guarantees or physical display
latency. Tests do not generate real keyboard/IME input.

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/runtime/main.exe
_build/default/examples/runtime/main.exe
_build/default/examples/runtime/main.exe --self-test
_build/default/examples/runtime/main.exe --shutdown-test
_build/default/examples/runtime/main.exe --last-window-test
_build/default/examples/runtime/main.exe --worker-backtrace-test
```

Native modes need graphical setup. Worker-backtrace mode initializes the runtime
then deliberately throws `Worker_test_failure`, checking the original
`fail_in_worker` backtrace survives cleanup. Shutdown mode shuts down immediately;
last-window mode closes during activation and relies on default exit policy.
Those flags precede the normal/self-test branch; do not combine unrelated modes.

For applications, keep durable producers in explicit application/conversation
scopes, use window scopes for window-owned work, and keep effects outside view
construction. Separate clock ticks from actual commits and frame callbacks from
presentation. See [bench](bench.md) and [input-core bench](bench_input.md) for
measurement-specific paths; compilation does not establish Linux GUI acceptance.
