# Loaded-list paint geometry diagnostic — OCH-17

On 2026-10-08, a temporary instrumented build of
`734fca1c4bb558c60149b04009dde2a6cc561e0e` exercised the unchanged OCaml
loaded-list workload on macOS 14.5 / Apple M1 Max arm64. This extends the
[earlier sampled scrolling investigation](list-scroll-diagnostics-och17.md).
It does **not** establish a rendering fix, complete traversal or performance
acceptance.

## What was observed

Forwarding native element wrappers recorded the final paint bounds of list rows,
placeholders and each populated row's content root. After painting the list, the
diagnostic checked:

- Ordered row bounds do not overlap by more than 0.5 logical pixels.
- Content roots stay within their row along the scrolling axis.
- Painted rows agree with `ListState.bounds_for_item` where that query returns
  bounds, within 0.5 pixels in position and size.

Each list paint starts with fresh records, capped at 256. Cumulative counts are
logged for each invocation, up to 100,000. Detailed geometry is retained for the
first three invocations, every thousandth and the first twelve anomalous ones.
The parser checks counter continuity and independently recomputes overlap for the
retained detailed samples. Neither run hit a record/log cap or had a list paint
without row records.

| Diagnostic | Result | List paints | Row paints | Placeholder paints | Geometry issue frames |
| --- | --- | ---: | ---: | ---: | ---: |
| 96-row smoke | Completed, 11.451 seconds | 317 | 1,864 | 114 | 0 |
| 10,000-row history | Incomplete, 225.656 seconds | 24,338 | 149,894 | 12,261 | 0 |

The full attempt failed while awaiting the rendered frame for row 2378 on the
backward traversal. Its failure observation records `active false`, an anchor
of `(2378, 0)`, 32 active rows and all 10,000 rows having been materialized.
The owner subsequently confirmed switching to another window during this run.
That aligns with the inactive-window observation and supports visibility as a
contributor to frame starvation; the diagnostic does not measure occlusion or
isolate it as the sole cause. Both collectors reaped
their owned children; the smoke exited 0 and the incomplete history exited 2.

These checks found no overlapping row rectangles in the observed paints. They do
not capture final display pixels, prove glyph-level clipping, measure presentation
cadence or exclude a transient outside the observed interval. The full attempt
did not reach the oversized-row growth or idle phase. Programmatic traversal
still makes immediate anchor jumps; that is distinct from ordinary wheel motion.
The original visible-jitter report remains open.

## Reproduction and provenance

Apply the archived `instrumentation.patch` to the base revision, then build with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_presented/list/main.exe
```

Copy that executable to an isolated diagnostic path and invoke
`scripts/measure_list_history.py` with `--build-profile release --presentation`,
the copied `--executable` and a fresh `--output`. The smoke additionally used
`--smoke --timeout 90`; the full attempt used `--timeout 600`. Exact commands and
the geometry summarizer are in the archive. No `--check-budgets` was requested:
logging, additional allocations and checks perturb performance.

The shared executable SHA-256 is
`603ffe5cfce9344ba8a2f436c13e0c3321afdfc797f6eff0f7080a86ec1e47ac`.
Both collector reports identify dirty source because of the temporary diagnostic.
The first build failed because its proposed JSON logger dependency was only
available in tests. The successful revision removes that dependency; both build
logs are retained. The temporary source was restored and the normal executable
rebuilt afterward. The diagnostic binary stays in ignored scratch storage.

The [archive](list-paint-geometry-och17/reports.tar.gz) and
[hash manifest](list-paint-geometry-och17/manifest.json) retain the exact patch,
final instrumented source, commands, parser, build logs, raw application logs,
collector reports and geometry summaries. OCH-17 remains In Progress. Further
investigation needs sustained visible-window evidence and, if the visual symptom
recurs, correlated frame captures; geometry checks alone are not a jitter verdict.
