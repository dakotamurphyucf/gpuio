# Pointer observer root reentrancy — OCH-17

The root's mouse-down/up focus observer used `Context::listener` even though it
needed only the separately owned focus manager. Native integrations can dispatch
input inside an existing root `WindowHandle::update`. The observer then attempted
a second mutable View lease and panicked. This is a GPUIO callback ownership
mistake; it requires no Bonsai, OCaml toolchain or GPUI fork change.

The observer now captures the focus manager directly and still defers observation
until after pointer dispatch. Explicit reveal requests, non-pointer focus reveal
and click-target stability retain their existing behavior. It adds no timer,
polling or synchronous OCaml callback.

## Evidence

Hosted run [37569701072](https://github.com/dakotamurphyucf/gpuio/actions/runs/37569701072)
at `90e7156b` fails native UI and document highlighting with the same double-lease
panic. Locally, the unchanged native UI reproduces it during Shift-click in
`selection_window::exercise`. A deterministic TestPlatform regression dispatching
down/up while the root is leased fails before the fix and passes afterward.
The adjacent regression also passes its held-click, explicit reveal and
non-pointer focus controls. TestPlatform is not physical desktop evidence.

Local macOS 14.5 / arm64 / Apple M1 Max checks after the production repair:

- Native UI passes selection, pointer policy, grid/style/focus, keyboard/Tab,
  replacement/reset and Unicode selection typography checks.
- Document highlighting gets past the original selection-policy panic, but its
  first run then fails a later wrapper-style assertion (expected zero highlights,
  observed two). That run lacks phase diagnostics; the exact transition and cause
  remain unknown. A subsequent instrumented run passes both Code and Markdown
  hover/press/focus states and the remaining document checks. This is a passing
  repeat, **not an established fix for the separate count failure**. The fixture
  now reports mode, phase, active/hovered window state and pointer position on
  failure; no assertion or timeout was weakened.
- A second instrumented document run passes after the final diagnostics change.
  Final UI and document runs both complete under the outer clipboard guard.
- A fresh Dune-built chat app passes the physical 300 ms held-click check: the
  cell center remains `(817.5, 549.0)` before down, during the hold and after up;
  the original row is selected and the child is closed/reaped.
- Full Dune `@all @runtest @fmt`, Rust formatting and documentation inventory
  checks pass (429 sources / 266 reviewed groups / zero pending).
- Full native unit suite: 1,073 passed, two existing ignored.
- Strict native all-target Clippy passes with `native-canvas-tests`,
  `native-image-tests` and `presentation-diagnostics`.

The two actual-window Rust fixtures also restore their GPUI clipboard snapshot
after a caught exercise panic. Subsequent local document execution uses the
stronger outer `mac_clipboard.preserved_clipboard` guard, preserving all eagerly
readable clipboard representations and reaping the child before restoration.
The initial failing UI run bypassed its old success-only clipboard cleanup;
its original clipboard contents were not saved externally and cannot be claimed
restored. No VoiceOver or desktop settings were changed.

The [archive](root-view-reentrancy-och17/reports.tar.gz) and
[manifest](root-view-reentrancy-och17/manifest.json) preserve before/after logs,
exact commands, initial test compilation error, all document results, source patch,
source/binary hashes and the held-click screenshot. All local build/test children
have exited. Reproduce using the isolated wrapper with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-canvas-tests,native-image-tests,presentation-diagnostics -- -D warnings
./scripts/gpuio exec dune build -j2 @all @runtest @fmt
./scripts/gpuio exec cargo fmt --all --check
```

Actual native integration targets are `native_ui` with `native-tests` and
`native_highlight_document` with `native-image-tests`; run them with bounded
process cleanup and clipboard preservation as shown in the archived driver.
These native-dispatch checks do not replace physical keyboard/IME/VoiceOver tests.

Current-source hosted/Linux checks, the
unexplained initial wrapper-style failure, physical presentation, broader catalog
and release qualification remain separate requirements.
