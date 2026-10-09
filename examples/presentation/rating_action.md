# Reducing rating intents against the latest configuration

[rating_action.ml](rating_action.ml) is pure application logic. Its `t` variant
contains Request of `Rating.Request.t`, Toggle_read_only and Toggle_disabled.
`initial` validates labelled Response quality with value 2; default maximum is
5. `configure` rebuilds the fixed demo configuration while preserving value and
setting the two policy flags. `apply` reduces each request with
`Rating.Config.apply_request`, or toggles one flag while preserving the other.
There is no native controller, I/O or Bonsai graph inside this module.

[main.ml](main.ml) installs `initial`/`apply` in `B.state_machine0`. A Bonsai state
machine owns persistent application state and returns a reactive model plus an
inject function producing deferred effects. Native `View.rating` emits a request;
its callback injects `Request request`, and `apply` operates on the latest model.
Four queued increases from 2 reach 5 by saturation, rather than all writing 3
from an old rendered value. Hover preview stays native and emits no request.
Bonsai `let%arr` then derives rating view/text from current configuration and
GPUIO submits it for native rendering; admission differs from physical display.

The [rating interface](../../lib/core/rating.mli) permits 0..maximum (0 means
unrated). Toggle on the already selected star clears to 0; out-of-maximum and
read-only/disabled requests are ignored. Read-only retains native focus/value
semantics, while disabled leaves traversal. The pure reducer checks current
policy even for a request queued before policy changed. `Or_error.ok_exn` treats
invalid fixed config as programming failure, not external-data recovery.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/presentation/main.exe
_build/default/examples/presentation/main.exe
_build/default/examples/presentation/main.exe --self-test
```

The executable owns this helper; [dune](dune) and [main.md](main.md) describe setup,
runtime, platform requirements and diagnostic boundaries. The self-test checks
burst saturation, clearing and a queued increase after read-only toggle; it is
not real keyboard/AX evidence. To use maximum 10, update both `initial` and
`configure` consistently, and preserve any customized star size/label when
reconstructing config. For async submission treat the rating as application data
and perform scoped Eio work outside view derivation; this reducer sends nothing.
