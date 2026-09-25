# Canvas resource transport check

Run `./scripts/gpuio exec dune exec examples/canvas_upload/main.exe` from the
repository root. The app starts GPUI's native runtime without opening a window.

It exercises the real OCaml/Rust bridge with a 20,000-item scene larger than one
message: ordered chunks, incomplete publication, rejection without changing the
accepted revision, generation reset, stale slot reuse, 63 pending requests plus
local backpressure, and clean shutdown. It uses the low-level `App.Expert.canvas`
surface to validate transport; this is not the eventual ergonomic canvas API or
a rendered-canvas example. No network, credentials or external files are needed.
