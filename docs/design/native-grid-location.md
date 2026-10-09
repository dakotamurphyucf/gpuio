# Native grid placement

OCH-41. `Grid_location`, also available as `Style.Grid_location`, supplies a
complete two-axis native grid location. The typed Core/wire/native implementation
is present and its background macOS geometry/GPU matrix passes. Full build/unit/lint validation also passes locally; rich Forms and public
gallery/consumer acceptance are not complete.

```ocaml
let module Grid = Style.Grid_location in
let two_columns = Grid.Axis.span (Grid.Span.of_int_exn 2) in
Style.create_exn [ Grid_location (Grid.create ~column:two_columns ()) ]
```

`Line` accepts nonzero integers in -1025..1025; `Span` accepts 1..1024.
Constructors reject invalid input, including machine-integer extremes, before a
style is created. Each `Axis` has typed `start` and `end_` edges: Auto, Line or
Span. `Axis.auto` sets both to Auto, `Axis.full` uses lines 1 and -1, and
`Axis.span count` sets both span edges. The complete location defaults to automatic
placement on both axes. Neither the constructor nor Rust duplicates the grid
layout engine; GPUI/Taffy resolves placement against the containing grid.

The [Forms design](form-composition.md#placement-contract-to-implement-first)
records source-derived normalization. Positive lines count from the first
explicit line, negative lines from the last. Reversed line endpoints swap;
equal endpoints occupy one track starting there. Two spans use the start span.
General grid style may create implicit tracks. These are normal in-flow rules;
absolute-positioned items use the containing area's edges for Auto endpoints.
Forms will additionally validate its items against the explicit column count.

## Atomic refinement and lifetime

`Grid_location` is one property, matching GPUI's whole-location style refinement.
A later declaration replaces **both** axes; a column-only constructor therefore
also sets the row to Auto. An interaction-state override follows the same rule.
Unsetting that state's declaration reveals the base declaration. Removing the
base declaration restores the receiving native component's default placement.
Parent row/column counts, track minima, gaps and other styles remain independent.

A location change updates the existing node's style. It adds no resource handle,
callback registry, layout round trip, timer or native owner. The Forms collection
will retain direct keyed grid children when its column count changes. Ordinary
containers are covered by the current native matrix; specialized widget root
style policies retain their own contracts and evidence requirements.

## Wire and admission

Style field 70 (`46` hex) contains a fixed record: column start/end, then row
start/end. Each edge is Auto tag 0, Line tag 1 plus a bin_prot int64, or Span tag 2
plus a bin_prot int64. Native decode rejects malformed tags and endpoints;
native transaction admission repeats endpoint validation for directly constructed
transactions. A rejected declaration must leave earlier operations, revision and
retained accounting unpublished. The native mapping writes the existing
`gpui::StyleRefinement.grid_location`; no dependency patch is introduced.

Capability 55 (`CAP_GRID_LOCATION`) requires a paired host. The current complete
mask is `9223372036854775807`; prior tags keep their encodings. An older host must
reject the required capability rather than silently ignoring placement.

| Value | Independent field/Hello bytes |
| --- | --- |
| Both axes Auto | `4600000000` |
| Full columns, Auto rows | `46010101ffff0000` |
| Two-column span, row lines 2..4 | `460202020201020104` |
| Hello requiring grid location | `0001fc0000000000008000` |
| Hello requiring the complete current mask | `0001fcffffffffffffff7f` |

## Local native evidence and remaining work

On macOS 14.5 arm64, the retained native fixture passes 20 geometry/GPU cases
across two parent widths, including both axes, signed/reversed/equal lines,
Auto and mixed span endpoints. It also checks whole-value hover replacement and
restoration, absolute-positioned Auto edges, removal of placement, and zero
retained tree bytes after unmount. GPU center pixels confirm the actual scene's
fill as well as recorded node bounds. The test creates a background window and
dispatches GPUI pointer movement; it is not OS keyboard/IME/accessibility evidence.
The initial fixture compile omitted its `gpui::point` import; adding it required
no production change. The completed run exited zero and closed/reaped its window
under a 180-second process-group watchdog.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_grid_location
```

Three Core expect tests pass endpoint limits, independent field/Hello bytes,
whole-value replacement and state-local unset. Three Rust protocol tests pass the
same independent fixtures, all four endpoint positions at their limits, malformed
and unknown tags, every message truncation and trailing bytes. Native session
validation rejects 32 malformed direct placements without publishing preceding
text changes, revisions or accounting. The refinement unit preserves parent
track counts while replacing both location axes.

Full Dune build/expect/format checks pass. Full Rust workspace tests pass,
including 423 native unit tests with two existing ignored. Strict all-target
Clippy, Rust formatting, workflow actionlint and the structural catalog audit
pass. Initial development checks caught the omitted wire-interface declaration
and two older aggregate-mask fixtures; those were corrected to the independently
specified new schema/mask before the final complete checks passed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --locked -j2 --all-targets --features gpuio-native/native-image-tests,gpuio-table-adapter/native-tests -- -D warnings
```

CI is configured to compile the fixture with the native image suite and run it
on macOS. The current hosted run still tests the earlier `e54d279` checkpoint;
it does not cover this addition. Public gallery/installed-consumer coverage
belongs to the next integration step. Forms remains pending.
