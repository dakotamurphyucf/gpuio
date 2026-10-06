# Native worker label-measurement type check

2026-10-06, macOS 14.5 arm64, source checkpoint `efc1711`.
The [draft outside-label plan](../design/sankey-label-placement.md) needs actual
native text metrics before Sankey geometry. A scratch probe checks whether the
pinned GPUI API permits moving the required owned context to a worker.

[Source](sankey-label-measurement-probe/source.rs.txt),
[exact compiler command/output](sankey-label-measurement-probe/compile.log) and
[checksums](sankey-label-measurement-probe/manifest.json) are retained.
The repository-isolated Rust compiler emitted library metadata successfully using
the already-built pinned GPUI rlib in `target/debug/deps`. Its hash-bearing filename
is a historical build artifact, not a portable dependency path.

The function captures `Arc<gpui::TextSystem>`, `gpui::Font`, an owned String and
font size in `std::thread::spawn`. Inside the closure it constructs its own
`WindowTextSystem` cache, shapes a line and returns its width. Compilation checks
the Send/lifetime constraints and the actual API signatures. No Window/App borrow
or OCaml value crosses that boundary.

**The function was not called.** This is not executed native text measurement,
a thread-safety stress test, a GUI result or outside-label implementation. Runtime
font/fallback behavior, cache/resource bounds, coherent font invalidation and
physical geometry still require the checks listed in the draft plan. The probe
changes no production code, dependency pin, default switch or public API.
