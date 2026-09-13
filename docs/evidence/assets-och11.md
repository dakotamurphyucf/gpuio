# OCH-11 encoded asset registration evidence

Local macOS, isolated OCaml 5.3/Core+Bonsai v0.17/Eio environment and pinned Rust
closure. Source descriptors landed in `706ba7c`; the native registry follows it.
No asset wire capability, FFI upload, decoding or GPU image rendering is claimed
by this checkpoint.

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
