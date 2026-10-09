# Runtime diagnostics and measurements

Read [main](main.md) for public runtime scopes, Bonsai clocks/lifecycles, shutdown,
and worker-backtrace scenarios. [Bench](bench.md) measures static/clock-dependent
native windows; [bench_input](bench_input.md) measures the OCaml driver with
simulated input/acknowledgement and opens no native window.

Each guide includes exact build/run commands, the actual source paths, measurement
units, and limits. Native runtime checks need graphical setup; the input-core
benchmark alone does not exercise the bridge or physical input. See
[development setup](../../docs/development.md).
