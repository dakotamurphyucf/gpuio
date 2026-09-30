# Animation opacity factors

OCH-41 adds `Animation.Property.Opacity_factor` to support rich Marker loading
without remounting content or wrapping it in a different layout. Marker integration
is still pending. The existing `Opacity` property continues to replace the styled
opacity, including its interaction overrides.

A factor multiplies the animated element's resolved base/interaction opacity.
It affects its background, border and children, and composes normally with ancestor
and descendant alpha. A missing base opacity means one; a missing state opacity
inherits the already multiplied base. State refinements with opacity replace that
base with their own value times the same factor. Thus a base of 0.8, hover of 0.6
and factor of 0.5 paint at 0.4 and 0.3 respectively. A constant factor of one
restores ordinary styling while preserving the animated node and its children.

```ocaml
let target factor =
  Animation.Target.create [ Opacity_factor, factor ] |> Or_error.ok_exn
in
let stage milliseconds factor =
  Animation.Stage.create
    ~timing:(Animation.Timing.tween
               ~easing:Animation.Easing.ease_in_out
               (Time_ns.Span.of_ms milliseconds)
             |> Or_error.ok_exn)
    ~target:(target factor)
    ()
  |> Or_error.ok_exn
in
let pulse =
  Animation.Program.create
    ~initial:(target 1.)
    ~repeat:Loop
    [ stage 1000. 0.6; stage 1000. 1. ]
  |> Or_error.ok_exn
in
View.animate_program
  ~key:(Key.of_string_exn "loading-content")
  ~style:(Style.create_exn [ Opacity 0.8; Grow 1. ])
  pulse
  [ content ]
```

This smooth two-stage curve is not a claim of exact cosine easing. Program controls,
retargeting from painted values, clocks, reduced motion, visibility and disposal
remain the existing native animation contracts. Stopping a loading effect should
replace this program with a zero-duration, one-shot target of one; cancellation
holds the last painted factor. A repeated pulse with initial one renders unchanged
style under reduced motion. No OCaml timer or per-frame callback is introduced.

Both legacy `Animation.Config` and advanced `Animation.Program` accept the new
property. Constructors and native admission reject nonfinite values, values outside
[0,1], duplicates and targets containing both `Opacity` and `Opacity_factor`.
There are twelve property tags but at most eleven properties in a valid target;
the existing bounded decoder limit remains eleven. Existing tag encodings and the
16 KiB advanced configuration limit remain unchanged.

The new property appends bin_prot tag 11 with one float64. Independent OCaml and
Rust fixtures encode factors 0, 0.5 and 1 as `0b0000000000000000`,
`0b000000000000e03f` and `0b000000000000f03f`. Capability bit 51
(`CAP_OPACITY_FACTOR`) requires a paired host/client rebuild. The current aggregate
mask is `4503599627370495`; Hello for just this capability is
`0001fc0000000000000800`, and for the aggregate `0001fcffffffffffff0f00`.
No GPUI dependency or fork changes are required.

## Validation status

Local macOS 14.5 arm64 validation passes the independent OCaml/Rust property/Hello
fixtures, finite/ambiguous-target rejection, bounded decoding, and native atomic
rejection in both animation APIs. The full local Dune build/expect/format suite, 413 enabled native unit tests,
native integration checks and all 245 Rust protocol tests pass. Two Linux
private-bus checks remain platform-specific. Strict native/protocol all-target
Clippy with `native-image-tests`, Rust formatting, workflow actionlint and the
structural catalog audit also pass; the catalog audit is not behavioral acceptance.

The background GPU fixture passes 15 base/factor combinations against independently
styled reference elements, including missing/zero opacity, ancestor/descendant
alpha, retained 120×80 layout, hover/press and restoration. A deterministic native
two-stage pulse reaches its low/high factors, reduces to ordinary styling without
further native wake requests and releases its owner on unmount. These are GPUI
input dispatch and GPU checks, not foreground OS keyboard/IME/accessibility tests.
The first fixture drafts omitted motion-policy initialization and complete subtree
removal; correcting the test harness required no renderer change or weaker checks.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --lib --tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --test opacity_factor
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --features native-image-tests --test native_opacity_factor
```

The native GUI process ran with a 90-second process-group deadline, closed its
background window and exited normally. CI builds the fixture on both platforms
and runs the bounded GPU check on macOS; hosted results are still pending.
Public Marker/gallery, installed consumer and release acceptance remain separate.
