# Choosing which avatar source to present

[avatar_mode.ml](avatar_mode.ml) defines only a closed variant `t = Initials |
Picture | Invalid_picture`. There is no Bonsai graph, native handle, effect or I/O
in this module. [main.ml](main.ml) stores it with `B.state Avatar_mode.Initials`;
[the main walkthrough](main.md) explains that state and the actual source-selection
trace. [avatar_assets.md](avatar_assets.md) explains the valid/invalid handles.

The three buttons request a mode through the state setter. In the final reactive
view, Initials supplies no asset, Picture chooses the embedded SVG handle when
available, and Invalid_picture chooses deliberately bad image bytes. Native
`View.avatar` retains its identity and selects fallback initials while source is
missing/loading/failed. Initials mode creates no artificial image failure/event.
The mode is application intent, not the native decoder's `Image.State.t`; those
observations separately update status. See [Avatar](../../lib/core/avatar.mli).

Build/run this support module through its owning executable, from the repository:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/presentation/main.exe
_build/default/examples/presentation/main.exe
```

No separate executable, assets or flags belong to this type. Runtime/toolchain
and platform prerequisites are in [main.md](main.md); the owning
[README](README.md) links diagnostics. To add another source mode, extend the
variant and exhaustive matches in `main.ml`, publish/own its source appropriately,
and keep source intention separate from actual decode readiness. Do not interpret
choosing Picture as successful decoding or persistence of a native asset handle.
