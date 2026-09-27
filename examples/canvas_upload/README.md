# Canvas resource transport and scoped API checks

Run `./scripts/gpuio exec dune exec examples/canvas_upload/main.exe` from the
repository root. The app starts GPUI's native runtime without opening a window.

It exercises the real OCaml/Rust bridge with a 20,000-item scene larger than one
message: ordered chunks, incomplete publication, rejection without changing the
accepted revision, generation reset, stale slot reuse, 63 pending requests plus
local backpressure, and clean shutdown.

The public `Gpuio_eio.Canvas` checks cover native publication, recovery from a
resource-history rejection, 1,000 coalesced resets without skipped generations,
image leases after asset retirement, rejection of new bindings to retired assets,
scope cancellation, 270 repeated registrations/releases, and progress through the
reserved adapter lane while raw requests are pending. Stage markers identify
failures; the internal deadline is 60 seconds (CI also applies 90 seconds).

No network, credentials or external files are needed. This validates registration
and ownership; it is not a rendered-canvas example or a graphical acceptance test.
