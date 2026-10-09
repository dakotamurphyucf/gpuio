# How `main.ml` tests encoded asset transport without a view

[README](README.md) · [Source](main.ml) · [Scoped API](../../lib/eio/asset.mli)

This executable is an integration diagnostic. It opens no windows and mounts no
Bonsai graph, image, or icon. `App.run ~exit_on_last_window:false` starts the native
runtime plus Eio UI domain until explicit shutdown. `App.scope` owns its task,
which has a 20-second timeout. Encoded publication is not pixel decoding.

`E` abbreviates `Bonsai.Effect`; `Asset` is the internal wire protocol and
`Scoped_asset` is the public `Gpuio_eio.Asset` adapter. `request` enqueues a UI
callback with `Scope.Expert.enqueue`, handles the request effect, maps its result
to an Eio promise, and awaits it. This sequences protocol replies without blocking
native callbacks. There is no `let%arr` because there is no reactive view; refs
record diagnostic completion rather than application model state.

The readiness probe submits `Begin (Png, 0L)`, retries `Not_ready` in 5 ms
intervals, and treats `Invalid_size` as proof negotiation finished. It then builds
more than 2 MiB of arbitrary binary bytes, including NUL/non-UTF-8, labels them
Png, and uploads ordered slices bounded by `Asset.max_chunk_bytes`. `ack` requires
Ack and `begin_` requires a resource ID. These bytes are deliberately not a valid
PNG fixture: successful Finish proves encoded storage, not successful decoding.

The script releases twice, checks slot reuse with increasing generation, and
requires old-generation release to return `Stale_handle`. It also checks incomplete
Finish, invalid negative offset, aborted upload state, four full source reservations,
resource-limit rejection, and quota recovery. Resource IDs are process-local
slot/generation identities, not persistent image names.

Next it creates a child scope and validated `Source.of_bytes`, then registers
through the public adapter. In one UI turn it occupies all 63 raw request lanes,
checks another raw request fails locally, cancels the owner, and releases the
registration again. The reserved adapter lane must still retire encoded storage.
Cleared `native_id`/released state are immediate local observations; four later
full reservations check native quota recovery. Another registration is cancelled
before completion; its user callback must not escape. One final registration
is left for terminal application cleanup.

Success checks zero commits and rendered frames, shuts down, and prints
`GPUIO_ASSET_UPLOAD_OK`. It validates real transport, quotas, generations,
cancellation, and shutdown, not native decoding, pixels, GUI input, or platform
presentation. No external file/network capability is needed.

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/asset_upload/main.exe
_build/default/examples/asset_upload/main.exe
```

There is no `--self-test` switch; every launch runs the diagnostic. It still starts
the native application backend, so do not assume arbitrary headless execution.
[Dune](dune) enables Jane Street/Bonsai PPX, but this module uses effect mapping
rather than Bonsai graph syntax. For ordinary applications, acquire real encoded
bytes with explicit Eio capabilities, use scoped registration, and borrow its
handle from a rendered image. Avoid copying raw lane/reservation tests into
production ownership policy; see [rendered images](../images/main.md).
