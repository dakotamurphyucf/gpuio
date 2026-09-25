# Container query evidence (OCH-26)

## Typed rule and codec foundation

Local macOS arm64, 2026-09-25. This checkpoint defines public abstract Branch_id,
Range, Predicate, Rule and Config modules, a pure reference selector, and a bounded
native configuration decoder. It does **not** mount responsive presentations,
change native focus/visibility, deliver selection events or advertise a capability.
Those remain required OCH-26 implementation work.

Half-open width/height ranges select the first matching rule, otherwise the
explicit default. Configurations allow 32 rules and 16 distinct branches with
128-byte UTF-8 IDs. The native decoder limits total standalone configuration bytes
to 4,096 and bounds nested lists and names before allocation. It validates positive
generations, finite/nonnegative/ordered bounds, unique IDs and branch indices.
Public construction rejects invalid inputs before conversion to the wire shape.

OCaml and Rust independently construct the same `container-query.hex` fixture:
default compact, wide at width >=480.25 and height >=200, then tall at height >=600.
Tests verify exact fractional lower/exclusive upper boundaries, height/width AND,
first-match overlap, rule-order changes, default and zero assigned size, invalid
nonfinite/negative sizes, UTF-8/NUL/name limits, branch/rule saturation and generation
validation. Rust also rejects every truncated fixture prefix, trailing data,
malformed UTF-8, oversized declarations before allocation, invalid indices/ranges
and configurations exceeding the total byte bound. Maximum admitted counts decode.

Targeted checks pass through the isolated jobs=2 toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test container_query -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-protocol --all-targets -j 2 -- -D warnings
```

Three OCaml expect scenarios and three new Rust tests pass. No graphical or Linux
execution is claimed. [The implementation contract](../design/container-queries.md)
describes the planned native layout, identity, observation and lifecycle behavior.


Final foundation checks also pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

Formatting changes were reviewed; no expect values were promoted. No application
windows were opened by these foundation checks. Hosted and mounted acceptance
remain pending.
