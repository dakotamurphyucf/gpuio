# Current-source streaming and typing qualification — OCH-17

One smoke warm-up and all three predeclared full runs pass on 2026-10-08 using
the unchanged CPU, input, presentation, pacing, content and memory budgets.
Each runs four independent 20 Hz streams for 120 seconds while the native composer
receives 1,200 real foreground keyboard events. All four streams complete 2,400
updates each; the final editor text exactly matches the expected 1,200 characters.
Final visible source blocks match their canonical hashes, and owned cleanup passes.

Executable source is `2562490ffee13059a6ca99ddc7108556f2a3972a`, SHA-256
`60388014ddb81f069323dfbb18baf29e3b071c928aeef6a45090a8e9b096b373`.
The observation checkout was clean `e4797b811d1c2605ac4594bb61c4eed64f666b5a`;
subsequent source differences were documentation only. Restored optimized build
logs and binary checks are retained. The environment is macOS 14.5 arm64, M1 Max,
32 GiB, internal display reporting 1728×1117 logical pixels at 120 Hz, AC power.
Display-mode metadata is not measured FPS.

| Trial | Draw p95 / p99 ms | Presented / attempted | Input→presentation p95 / p99 ms | Submission→presentation p95 / p99 ms | Peak RSS bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 001 | 4.518 / 5.542 | 3,871 / 3,872 | 30.589 / 32.850 | 22.905 / 27.591 | 134,529,024 |
| 002 | 4.497 / 5.603 | 3,762 / 3,763 | 29.065 / 32.358 | 22.823 / 27.214 | 137,854,976 |
| 003 | 4.641 / 5.698 | 3,722 / 3,723 | 30.491 / 35.226 | 23.724 / 28.557 | 132,333,568 |

Every full run has more than 5,000 draws and exactly 1,200 paired native input
samples. Each retains one initial input-free zero-time drawable, with first
positive presentation at 28.114, 69.347 and 69.218 ms, within the existing 100 ms
transition bound. There are no later zeros, lost/invalid/pending samples, histogram
overflow or raw-trace truncation. All 121 periodic observations and the final
observation report active/visible; this is sampled visibility, not continuous
proof. Peak active history is ten rows in each run.

The driver verifies foreground ownership before every key and restores the original
input source after each run, including the warm-up. No VoiceOver or clipboard
operation was used. All children and the batch's keep-awake process were reaped.
No local compiler or second owned GUI ran concurrently; lightweight source reads,
scratch preparation and hosted-job polling/downloads continued. Reports preserve
that real desktop environment rather than claiming laboratory isolation.

```sh
python3 scripts/measure_streaming_typing.py --build-profile release --presentation \
  --check-budgets --timeout 1200 \
  --executable _build/default/examples/performance_presented/streaming/main.exe \
  --output scratch/agents/root-20261007-access-check/streaming-2562490f-001
```

Use suffixes `002` and `003` for the other full trials; warm-up substitutes
`--smoke` for `--check-budgets`. Every full report has `complete=true`, child/driver
exit zero, empty `budget_failures` and `input_source_restored=true`. No failed
trial was replaced.

[Summary](presentation-streaming-current-och17/summary.json),
[raw archive](presentation-streaming-current-och17/reports.tar.gz) and
[SHA-256 manifest](presentation-streaming-current-och17/manifest.json) retain all
four reports, application/driver logs, input-source recovery records, batch plan
and build logs. All 23 archive members were read back and hash-verified.

These are Metal-reported presentation times starting at native input dispatch,
excluding the preceding OS queue, keyboard hardware and photon latency. This
qualifies the defined workload on this Mac, not arbitrary 120 FPS applications.
Outstanding list findings, instrumentation overhead, active-collector retirement,
hosted checks and other release requirements remain. Earlier source results remain
in [the historical streaming evidence](presentation-typing-full-och17.md).
