# Chart resource integration check

Read the [source walkthrough](main.md) for the exact transport stages, effect
scheduling, ownership, commands, and diagnostic limits.

Run `./scripts/gpuio exec dune exec examples/chart_upload/main.exe` on macOS.
This opens no windows, but uses the real native application event loop and OCaml
UI domain. It is an integration check for the public `Gpuio_eio.Chart` resource
API, not a rendered chart showcase.

It publishes 100,000 points with explicit gaps, coalesces 1,000 updates and a reset,
checks malformed native publication and recovery, cancels the data scope, cycles
270 registrations, verifies bounded request lanes and completes queued requests
during application shutdown. A 60-second internal deadline bounds failures.
Success prints `GPUIO_CHART_UPLOAD_OK`.

The scoped handle can be shared by chart views without copying dataset payloads
into each view. Scope release invalidates it. Rendering, selection, and accessibility are separate from this check; see the
[rendered workload](../chart_stream/README.md) and its actual evidence.
