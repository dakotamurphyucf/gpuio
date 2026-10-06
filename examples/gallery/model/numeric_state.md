# Numeric state: reduce rating requests against current policy

[numeric_state.ml](numeric_state.ml) and its [interface](numeric_state.mli) own the
application-controlled rating and shared read-only preference in the gallery's
Numbers & codes page. This pure model does not own the page's native number,
slider or verification-code editors. It creates no Bonsai graph, Eio work or
native controller; [numeric_page.ml](../numeric_page.ml) supplies those layers.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Numbers & codes**, change the rating/maximum/star size, toggle read-only
and try step-down filled stars. No independent executable, asset or model diagnostic
flag exists. See [development](../../../docs/development.md) for toolchain setup;
this review adds no native input, IME, platform or appearance acceptance.

Read `t`, `initial`, `configure`, then `apply`. Abstract `t` stores a validated
`Rating.Config.t` plus custom-color and step-down Booleans, both initially false.
Initial rating is 3 of 5 at the library's default 24-pixel size, enabled and editable.
The public [rating interface](../../../lib/core/rating.mli) defines request variants,
maximum 1–32, value 0–maximum and zero as unrated.

`configure` reconstructs the config, preserving all unspecified size/disabled/read-only
fields and clamping current value down if maximum shrinks. The validated config
constructor itself rejects invalid values rather than clamping; this helper applies
that explicit adaptation before constructing. Record update preserves custom-color
and step-down preferences too. Toggle_read_only, Toggle_rating_disabled and size/
maximum actions use this helper; maximum cycles 5→10→1→5, size cycles 24→36→16→24.
Color/step-down toggles change only their Boolean preferences.

`Rate request` applies the typed intent to the latest config with
`Rating.Config.apply_request`. Set/Increase/Decrease and ordinary Toggle retain
library semantics; disabled/read-only and out-of-current-maximum requests are ignored.
With step-down enabled, Toggle index at or below current value is first transformed
to Set (index−1). This computes from current model state, not the value that happened
to be rendered when the event was queued. Fixture `ok_exn` wrappers unwrap validated
bounded configuration/requests; no arbitrary raw numeric action bypasses the contract.

The caller wraps `State.apply` in `B.state_machine0`: a reactive model plus injection
effect. `B.map` derives read-only for Number_preview, Slider_preview and OTP config,
and rating for `V.rating`. Its `let%arr` reads model/palette to derive views.
`V.rating ~on_request` injects Rate; native hover previews stay native and emit no
request. Native star geometry/focus belong to GPUIO, while the selected rating is
application-owned. Custom colors become `Rating.Appearance` in the caller, separate
from the model's numeric config.

For a concrete trace, enable step-down at rating 3 and click filled star 3. Native
Toggle 3 arrives, the injection effect reduces it to Set 2, and Bonsai derives a
2-of-5 config/readout. Set read-only before a queued Increase reduces: the reducer
passes it to the now-read-only config and value stays 2. Cycle maximum to 10 then 1:
configure clamps value to 1; a delayed Toggle 3 cannot restore an out-of-range rating.
The model does not issue editor commands or start a timer for these updates.

The [existing gallery expect test](../../../test/gallery/gallery_test.ml) checks
appearance/size retention, step-down, read-only, smaller maximum and disabled requests.
No tests were run for this guide. State lifetime belongs to its page graph; there
is no model cleanup or native handle. To add another maximum preset, extend the
cycle with values 1–32 and keep explicit value clamping. To change hover colors,
change caller appearance without treating a hover as an accepted Rate action.
