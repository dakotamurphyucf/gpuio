# OCH-11 decorative button icons — local evidence

Public API and native tree/rendering integration are implemented on the local
OCH-11 branch. No hosted/Linux result or merge is claimed by this checkpoint.

Verified locally on macOS:

- Core expect tests reject raster decorations and empty icon-only labels; verify
  decorative/no-callback children, stable trailing icon/button handler after leading
  removal, and disabled action removal. Ordinary buttons keep their existing API.
- Native tree tests reject icon/slot callbacks, named icon children, nonempty slot
  text and invalid child shapes atomically. Nonstructural Bind/SetImage updates are
  included, along with valid source replacement and icon removal.
- The focus:false production-window test checks GPU icon colors at measured positions,
  leading/trailing pointer clicks producing exactly one parent Press, Enter activation,
  and disabled suppression. Input is synthetic GPUI dispatch, not OS keyboard automation.
- AppKit exposes AXButton for the labelled composite. Native command registry changes
  update the accessible label and move the trailing icon with the longer visible text.
  Clicking an icon emits one CommandInvoked with the current generation and the button
  as its source; disabling the registry command suppresses it. An icon-only button
  with one populated slot exposes its explicit AXButton label and activates once.
  Existing mounted icon
  leases survive reparenting after asset registration retirement, then dispose cleanly.
- The public `examples/images` self-test passes raster/SVG/icon modes. Icon mode now
  creates ordinary, icon-only and command buttons, and removes a trailing decoration
  after asset retirement. This checks public FFI/tree/lifecycle integration. It does
  not automatically click the public example; native input tests provide that coverage.
- Full Dune `@all @runtest @fmt` and feature-enabled native all-target Clippy with
  warnings denied pass, along with `cargo test --locked --workspace`. The final
  production-window run includes the icon-only button case and passes.

Commands:

```sh
./scripts/gpuio exec dune build @all @runtest @fmt
./scripts/gpuio exec cargo test --locked --workspace
./scripts/gpuio exec cargo test --locked -p gpuio-native --test button_icons
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-image-tests --test native_image_views --no-run
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --features native-image-tests --all-targets -- -D warnings
```

The reported native binary runs under a 45-second child-process timeout; public
example modes use 35 seconds each. Logs are `button-icons-*.log` in the implementing
agent's ignored ticket scratch directory. The [contract](../design/native-controls.md#decorative-button-icons)
records the API, fixed decorative slots, native label ownership and capability bit.
Remaining OCH-11 theme/state/transitions, scrolling/lifetime acceptance and consolidated
macOS/Linux gates/merge still need completion.
