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
