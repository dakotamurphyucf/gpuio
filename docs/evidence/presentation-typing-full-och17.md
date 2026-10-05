# Full streaming and typing presentation qualification — OCH-17

Three optimized runs pass at clean `e2939be92cafbfac6a8e1a49727f9c8e1c83b601`
on macOS 14.5 arm64, Apple M1 Max, 32 GiB RAM, internal display reporting
1728 × 1117 at 120 Hz. Each runs four 20 Hz streams for 120 seconds while
delivering 1,200 OS keyboard events. All four streams produce 2,400 updates;
the final editor text contains exactly the expected 1,200 characters.

Each run pairs all 1,200 CPU input samples with native Metal presentation
samples. The measurement starts at native input handling and ends at Metal's
host-clock presentation timestamp; it excludes device and photon latency.
No other owned GUI or local compiler ran concurrently. The input source was
restored, resource cleanup passed, and all three child processes exited zero
and were reaped. VoiceOver was neither operated nor configured.

| Run | Presentations / attempts | Startup bound observation | Input-to-presentation p95 / p99 | Submission-to-presentation p95 / p99 | Peak RSS bytes |
| --- | --- | --- | --- | --- | --- |
| 001 | 3,827 / 3,828 | 77.944 ms | 30.015 / 32.489 ms | 23.216 / 27.476 ms | 135,544,832 |
| 002 | 3,864 / 3,865 | 61.078 ms | 30.310 / 32.866 ms | 23.544 / 27.689 ms | 143,589,376 |
| 003 | 3,756 / 3,757 | 77.639 ms | 29.770 / 33.178 ms | 23.331 / 27.607 ms | 138,428,416 |

Each run retains one initial, input-free zero-time outcome under the previously
declared [startup-transition policy](../design/metal-presentation-qualification.md#idle-to-active-correction--2026-10-05).
There are no later zeros, other losses, invalid clocks, pending callbacks,
histogram overflows or trace truncations. All 121 periodic observations per run,
plus the final observation, report active and visible; these are samples, not
continuous visibility proof. Raw traces and cumulative histograms are retained.
All content, CPU, presentation and resource budget gates pass.

All runs use executable SHA-256
`9f178debf9b16df7b511c7d476a9f23b69e19c3a8ef5c0fe36b58a63656f06ba`.
The release build and cached recheck are recorded in the
[build evidence](presentation-list-full-och17.md). The intervening commits only
change documentation and notice provenance, not executable inputs.

```sh
# Run separately for NNN = 001, 002, 003, without concurrent GUI workloads.
python3 scripts/measure_streaming_typing.py --build-profile release --presentation \
  --check-budgets \
  --executable _build/default/examples/performance_presented/streaming/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-typing-full-NNN
```

Evidence: [summary](presentation-typing-full-och17/summary.json),
[artifact hashes](presentation-typing-full-och17/manifest.json),
[run 001](presentation-typing-full-och17/run-001-artifacts.tar.gz),
[run 002](presentation-typing-full-och17/run-002-artifacts.tar.gz),
[run 003](presentation-typing-full-och17/run-003-artifacts.tar.gz).
Each archive includes the original application log, report, input-source recovery
record and driver output.

This qualifies this workload on this local Mac. It does not resolve the
[full-list startup failure](presentation-list-full-och17.md), qualify table or
document presentation workloads, measure collector overhead or complete the
release's broader resource/platform requirements. OCH-17 remains open.
