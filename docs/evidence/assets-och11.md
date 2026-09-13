# OCH-11 encoded asset registration evidence

Local macOS, isolated OCaml 5.3/Core+Bonsai v0.17/Eio environment and pinned Rust
closure. Source descriptors landed in `706ba7c`; the native registry follows it.
The wire integration below adds encoded registration; decoding and GPU image
rendering remain unimplemented.

Five native registry tests pass:

- A binary source larger than 2 MiB assembles through chunks no larger than
  256 KiB. No staging prefix can be acquired; final bytes and declared format
  are preserved exactly. This tests the store, not an actual transport exchange.
- Invalid offsets, empty/oversized chunks and incomplete finish reclaim staging.
  Old-generation append/finish/release operations cannot affect a reused slot.
- Retired registration handles cannot acquire new readers. Existing readers remain
  valid and keep their bytes charged until their last lease drops; quota cannot be
  bypassed by releasing a handle while a view/worker still owns its data.
- Shutdown cancels staging, retires available sources, blocks new registration and
  acquisition, and allows existing readers to finish before their quota is reclaimed.
- Encoded size, concurrent-upload, entry and generation-exhaustion bounds reject
  requests without aliasing or excess reservation.

The native session test also passes negotiation gating, application-owned reuse
across window generations, retired-reader accounting and terminal shutdown. It
uses native session state, not actual windows or an Eio scope.

Validation passed:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --lib asset_store::
./scripts/gpuio exec cargo test -p gpuio-native --test session
./scripts/gpuio exec cargo clippy -p gpuio-native --all-targets --features native-tests -- -D warnings
./scripts/gpuio exec cargo test --workspace
./scripts/gpuio exec dune build @runtest @all @fmt
```

Three existing system-libwayland tests remain ignored on macOS and require the
consolidated Linux build/unit gate. Logs are under the personal scratch directory:
`assets-store-final-unit.log`, `assets-store-session.log`,
`assets-store-final-clippy.log`, `assets-store-workspace.log`, `assets-store-dune.log`.

Next acceptance is the correlated wire upload/cleanup path and independent binary
fixtures, followed by scoped Eio registration, native decode/cache budgets and
image/icon views. The [asset design](../design/assets.md) records the implemented
registration lifetime separately from those remaining interfaces.

## Correlated FFI upload

Following registry commit `43aed0d`, independent OCaml/Rust fixtures agree on all
nine formats, all response errors, multi-byte integer encodings and opaque chunk
bytes. Truncations, trailing data, invalid correlations and oversized chunks are
rejected; Rust bounds a declared chunk length before allocation. Two native
mailbox/session tests verify reserved replies under input/response pressure,
window-slot independence, prefix reclamation and shutdown rejection.

The windowless `examples/asset_upload` program passes through the actual native
host and public Eio runtime: >2 MiB split into <=256-KiB chunks, finish/release,
slot reuse and stale-release rejection, incomplete upload and invalid-offset
reclamation, 64-MiB quota exhaustion/recovery, and clean application shutdown.
It asserts zero view commits/render callbacks. This proves upload/control flow;
exact bytes are checked compositionally by the fixtures and native registry tests.
It does not claim decoded rendering or scoped public cancellation.

Commands passed locally on macOS:

```sh
./scripts/gpuio exec cargo clippy --workspace --all-targets --features gpuio-native/native-tests -- -D warnings
./scripts/gpuio exec cargo test --workspace
./scripts/gpuio exec dune build @runtest @all @fmt
_build/default/examples/asset_upload/main.exe
```

Logs: `assets-wire-clippy.log`, `assets-wire-workspace.log`,
`assets-wire-dune-final.log`, `assets-wire-native-upload-final.log` in the personal
scratch directory. An earlier full Dune run found two nonexhaustive example event
matches; both now explicitly handle the new response and the full rerun passed.
Three system-libwayland tests still await Linux. No hosted run has been started
for this checkpoint, following the accepted complete-local-scope-first workflow.

## Scoped registration checkpoint

The public `Gpuio_eio.Asset.register` effect and UI-domain registry now attach
encoded registrations to explicit application scopes. Five expect tests cover:

- Cancellation before Begin, with Begin/each chunk/Finish in flight, and after
  ready delivery: no cancelled user callback, exact late-ID retirement, no further
  chunks after cancellation, and zero retained registry/source accounting.
- Exact chunk bytes/offsets, source-reference release before Finish, idempotent
  retirement and retry after transient cleanup admission pressure.
- Eight-upload/64-MiB source admission, foreign-root rejection, queue cancellation
  and terminal closure.
- Native Begin/Append errors, known-ID cleanup and suppressed late shutdown reply.
- 1024 ready registrations, rejection at the metadata bound, and one acknowledged
  retirement for every registration at scope end.

The actual windowless native example additionally completes a >2-MiB public scoped
registration, fills all 63 raw request lanes, cancels its scope and immediately
observes released public state. Four subsequent 16-MiB native reservations prove
that its encoded bytes were reclaimed despite raw traffic pressure. It also checks
cancel-before-upload callback suppression and clean shutdown with a ready scoped
registration. Exact in-flight cancellation cuts are deterministic controller tests;
the native test does not claim those timing cuts were forced at the OS boundary.

Full local `dune build @runtest @all @fmt` and the windowless executable are the
validation gate for this OCaml-only checkpoint. Logs: `assets-scoped-unit.log`,
`assets-scoped-dune-all.log`, `assets-scoped-native-final.log`. Rendering, decoded
cache limits, pure image handles and pixel/theme/scale acceptance remain pending.

## Native raster decoder checkpoint

Four local native tests cover the eight raster format families, actual decoded
BGRA pixels (JPEG with an explicit lossy tolerance), GIF frames/delays/count limit,
malformed data and oversized independent PNM/GIF headers, and an independently
assembled JPEG EXIF orientation segment. A broken later GIF frame rejects the
whole result. The ICO fixture uses RGBA PNG content as required by the pinned
ICO decoder; an initial RGB fixture correctly failed and was corrected.

`image = 0.25.10` is now a direct dependency, reusing the existing locked package
and feature closure. No decoder version changed. Commands passed locally for this checkpoint:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --lib asset_decode::
./scripts/gpuio exec cargo clippy -p gpuio-native --all-targets --features native-tests -- -D warnings
./scripts/gpuio exec cargo test --workspace
./scripts/gpuio exec dune build @runtest @all @fmt
```

Logs: `assets-decode-tests-final.log`, `assets-decode-clippy.log`,
`assets-decode-workspace.log`, `assets-decode-dune.log`. These are in-memory native
pixel tests, not GPU rendering or worker/cache acceptance. The design separates
strict retained-output bounds from best-effort codec allocation limits and
pending aggregate worker/cache reservations. No new wire capability is advertised.

## Decoded cache/work-ticket checkpoint

Controller tests cover shared decoding and actual BGRA output returned from a
background OS thread; warm-cache independence from encoded registration lifetime;
32 queued/two running jobs; abandoned work/completion reclamation; late replacement
and shutdown rejection; image references remaining charged after eviction; and
pixel admission before dispatch. The capacity accounting test uses small real pixel
results with synthetic charged sizes to exercise 256-MiB admission without allocating
that much in a unit test. It does not claim a measured process-RSS bound.

Further cases cover foreign cache/handle/work identity despite equal numeric IDs,
256 live/warm plus 256 pending-retirement metadata bounds, recovery after eviction
handoff, and immediate last-owner cancellation before another cache turn. Tests
use both real decoder results and controlled completion timing. No GUI window or
actual GPU atlas eviction is exercised by this checkpoint.

Seven controller tests, native all-target Clippy, full Rust workspace tests and
full Dune @runtest/@all/@fmt passed before the final immediate-cancellation
refinement. That refinement adds the eighth test and a direct shared atomic
cancellation flag; all eight final targeted tests and Clippy pass and are logged
separately. Logs:
`assets-cache-final-unit.log`, `assets-cache-final-clippy.log`,
`assets-cache-workspace.log`, `assets-cache-dune.log`,
`assets-cache-cancel-unit.log`, `assets-cache-cancel-clippy.log`.

The host integration must still schedule and drain the tasks, deliver observable
state to mounted views, and evict each used window atlas. The inspected pinned
Metal and Linux WGPU renderer constructors create per-window atlas instances;
this is source evidence for the disposal design, not backend execution evidence.

## Native host and GPU readback checkpoint

The actual macOS `native_images` test passes with `WindowOptions.focus = false`.
It checks background decode/completion refresh, one shared decoded image across two
windows, exact GPU-readback pixels, continued rendering after encoded registration
retirement, window-close accounting, replacement, and shutdown with two outstanding
jobs/results. After shutdown, worker reservations, delivering/queued completions
and managed atlas counters are empty. This test uses native encoded-store leases;
it is not yet a public OCaml image-view/FFI rendering example.

The atlas cleanup assertion is behavioral. A test-only RenderImage reuses an old
image ID with different pixels: before eviction the backend returns the old tile;
after shutdown evicts the key, a fresh paint/readback returns the new pixels. The
test retains original CPU image references during this check. Production code never
reassigns image IDs. The diagnostic repaint deliberately repopulates an unmanaged
tile, then the test closes its window; zero service counters refer to managed images.

Two initial fixture/probe issues were resolved without patching GPUI:

- A magnified single texel sampled neighboring atlas padding under GPUI's linear
  filtering. The final fixture uses a solid 4x4 image and exact interior RGBA checks.
- `Window::has_image_atlas_entry` delegates to `PlatformAtlas::contains`, whose
  default false implementation is not overridden by the real Metal atlas. It was
  replaced with the old-tile/new-upload behavioral assertion above.

Verified local commands:

```sh
./scripts/gpuio exec cargo clippy -p gpuio-native --all-targets --features native-image-tests -- -D warnings
./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --test native_images --no-run
# Execute the reported native_images binary with a 45-second process timeout.
```

Logs: `assets-host-clippy.log`, `assets-host-native-build-verified.log`,
`assets-host-native-verified.log`. Workspace/Dune/windowless upload regressions
are recorded separately in `assets-host-workspace.log`, `assets-host-dune.log`,
`assets-host-upload-regression.log`. The CI workflow now compiles/lints this optional
readback target on both platforms and executes it on macOS; no hosted run or Linux
GPU result is claimed. SVG, native image semantics/styles/events and public pure
view handles still require integration before this is a completed image feature.

The full local Rust workspace, Dune @runtest/@all/@fmt and windowless >2-MiB
FFI/scoped-upload regression passed with the host shutdown integration. Final
review moved deferred-pump coalescing into the deferred callback and registered
windows observing a shared loading handle before readiness. The final native GPU
test explicitly covers that loading-window case; it and native-image-tests
all-target Clippy pass (`assets-host-sharing-native.log`,
`assets-host-sharing-clippy.log`). The preceding coalescing check also passed
(`assets-host-final-native.log`). No further hosted verification is claimed.
