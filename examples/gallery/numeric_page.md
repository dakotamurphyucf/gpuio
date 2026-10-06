# Compose native numeric editors with controlled rating state

[numeric_page.ml](numeric_page.ml) and its [interface](numeric_page.mli) assemble
sliders, numeric drafts, OTP cells and a rating. Read policy/state, child components,
OTP config/controller, rating request adapter and final view. `B = Bonsai.Cont`
is reactive graph wiring; `V` views; `Otp_controller` owns a native editor;
`State` is the pure [Numeric_state](model/numeric_state.md) reducer. The unused
`E` alias does not imply an Eio task or effect-based authentication flow.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section otp
python3 scripts/test_gallery.py --section rating
```

Choose Numbers & codes. [Slider](slider_preview.md) and [Number](number_preview.md)
walkthroughs cover their independent components/sections. [README](README.md) records
native/platform evidence; these commands are documented, not newly executed.
All initial values are local fixtures; OTP completion is not authentication.

## Shared read-only policy and separate native owners

`state_machine0` reduces typed Numeric_state actions against latest state. read_only
derives from rating config and is passed to slider/number children and OTP config,
so one switch controls their input policy. Rating-specific disabled is separate.
`B.map` makes derived config/action functions; `let%arr` combines child outputs,
controller snapshot and palette for presentation without eagerly executing requests.

`Otp_input.Policy.create` fixes length 6 with default digit alphabet. `Otp_controller.create`
starts Value.empty and one labelled exact-window placement. Rust owns text/preedit/
selection/history; this component shows only is_complete from last accepted snapshot,
which may lag native input. Policy is immutable per placement; changing length/
alphabet needs deliberate remount. Grouping (2 versus 1), cell width (40 versus 28),
masking, read-only and palette appearance update current placement, not initial text.
Masked snapshots still contain real text, but this page displays no code value.
[OTP controller](../../lib/eio/otp_input.mli) and [value contracts](../../lib/core/otp_input.mli)
separate accepted editing from a server verification decision. There is no network
submission, secret persistence or native command call in this page.

## Rating intents remain application-controlled

`V.rating` gets `State.rating` and a callback injecting Rate request. Hover preview
is native and does not update rating. [Rating](../../lib/core/rating.mli) validates
value 0–maximum, maximum 1–32 and star size 8–128; `apply_request` ignores unavailable/
out-of-current-range queued intents and saturates increases/decreases. Read-only
retains focus but emits no requests, whereas disabled leaves keyboard traversal.

The reducer starts at 3 of 5, cycles maxima 5 → 10 → 1 → 5 (clamping stored value), sizes
24 → 36 → 16 → 24 and optional active/inactive palette colors. Step-down changes a Toggle
on a filled index into Set(index - 1); otherwise default toggle sets/clears that star.
These policies execute against latest state, not a Boolean captured by the last
render. Optional colors override foreground independently; changing appearance
is not value reset. The text output reports validated application value/maximum.

Trace: enable step-down → click a filled star → native emits Toggle index → reducer
translates against current value and applies request → `let%arr` supplies new config
and label. Another switch can make controls read-only before a queued request,
which the reducer rejects safely. These stars do not write preferences externally.

Page departure unmounts native editor leases; branch model values can survive page
visits, but this page does not mirror OTP text into application storage for remount.
There is no preview resource scope or Eio producer here. To adapt, keep validated
numeric state/requests distinct from native drafts, and add explicit scoped server
verification for a real OTP. Use the child editor guides for draft/IME commands;
compilation/completion text alone does not establish native input or authentication.
