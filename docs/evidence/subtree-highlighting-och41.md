# Subtree highlighting — OCH-41 foundation

This checkpoint implements validated configuration and a windowless native query
kernel. It does **not** implement mounted highlighting, scope observations,
explicit-range projection, native document painting or gallery acceptance. No new
capability, node kind or event tag is advertised. OCH-41 remains In Progress.

The [design](../design/subtree-highlighting.md) records the pinned GPUIX comparison,
UTF-8 byte/range convention, independent match indices and virtualization offsets,
nested override policy, theme resolution and remaining ownership work.

Implemented:

- Public `Highlight.Query/Range/Appearance/Spec/Config` constructors with abstract
  OCaml types and matching protocol validation. Maximum 16 specs, 4096 aggregate
  ranges, 4096 bytes/query and 256 KiB encoded configuration. Indices preserve
  nonnegative signed 64-bit values; active-index comparison cannot overflow.
- Independent Rust/OCaml configuration bytes, malformed UTF-8/option/Boolean tags,
  every truncation, trailing bytes, semantic bounds and maximum aggregate size.
  Theme defaults multiply accent alpha; explicit colors preserve their alpha.
  Cosmetic changes leave matcher identity unchanged.
- Reusable native compiled queries with a streaming KMP matcher. It recovers
  original UTF-8 positions after scalar lowercasing, supports whole-word checks,
  adjacent chunks and line barriers, and retains only bounded match records.
  It allocates no lowercase source copy or source-sized origin map.
- Shared source-byte, group, chunk and stored-match budgets. Cancelled or
  work-limited requests return a typed failure; storage limits retain exact counts
  and explicitly indicate missing ranges. Empty/infinite empty-chunk iterators
  cannot bypass work limits. These checks do not substitute for scheduler-wide
  memory admission or mounted-scope resource accounting, which remain pending.

Local verification on macOS 14.5 arm64, using the repository-isolated toolchain:

| Command | Result / boundary |
| --- | --- |
| `./scripts/gpuio exec dune runtest test/highlight` | Four expect tests pass. |
| `./scripts/gpuio exec cargo test -p gpuio-protocol --test highlight --locked -j2` | Four configuration/codec tests pass. |
| `./scripts/gpuio exec cargo test -p gpuio-native --lib highlight_search --locked -j2` | Final eight matcher tests pass, including 123244 comparisons against an independent allocating oracle, all scalar chunk splits, 100000 matches with bounded storage, reuse, empty groups/chunks and cancellation. |
| `./scripts/gpuio exec dune runtest` | Full suite passes after configuration/kernel addition. |
| `./scripts/gpuio exec cargo test -p gpuio-native -p gpuio-protocol --locked -j2` | 693 tests pass before the final compiled-query/empty-chunk refinements; those refinements pass the eight-test focused matcher rerun above. |
| `./scripts/gpuio exec dune build @fmt` | Passes. |
| `./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --features native-tests --all-targets --locked -j2 -- -D warnings` | Passes; the range-vector fixture module has a documented narrow allowance for Clippy's single-range-array ambiguity lint. |
| `./scripts/gpuio exec cargo fmt --all --check` | Passes. |

Two initial expectation errors were corrected deliberately: Core's S-expression
printer escapes the non-ASCII bytes in `café`, and a hand-counted range after a
Greek letter used the wrong byte position. Cross-language bytes and matcher
reference comparisons pass; neither failure involved a GUI or user interaction.

Next acceptance: ordinary-text grouping and explicit-range validation/projection,
bounded worker admission/cancellation and typed pending/ready/invalid/capacity
observations, retained View/reconciliation/session integration, text/selection and
Markdown/code/diff paints, nested scopes, virtualization offsets, independent
windows and a public gallery find bar. No native interaction, hosted CI or Linux
desktop acceptance is claimed by this windowless checkpoint.
