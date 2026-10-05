# Focused and unfocused idle — OCH-17

Three full optimized local macOS runs pass at source
`1180cbb6e834882f138c91c1ed83667d187d1748`, with a clean tracked tree at each
launch. Each had a separate two-second-per-phase smoke warmup, followed by
60 seconds focused and 60 seconds unfocused after two-second settling intervals.
All five native histogram counts were zero in all six full intervals, with no
dropped input timestamps or activation transitions. Editor focus then blur,
independent AX foreground observations, native visible observations and final
owned-resource/queue retirement passed. No full attempt was discarded.

Hardware: Apple M1 Max, 32 GiB, macOS 14.5 arm64, built-in display reporting
1728×1117 logical / 3456×2234 physical at 120 Hz. The ordinary opaque window
requested 1200×800 logical pixels. All three reports record AC power, charging,
and no recorded thermal/performance warning. One owned GUI workload ran at a
time without a simultaneous compiler; read-only review and remote CI continued.
The exact executable SHA-256 was
`a12d50677ae58d618419e86e7ec8682f172e0056b3a835828e9c27a58982f1e2`.

| Run | Focused seconds | Unfocused seconds | Draw counts focused / unfocused | Whole-child CPU seconds | Peak RSS bytes |
| -- | --: | --: | --: | --: | --: |
| 001 | 60.008387 | 60.014398 | 0 / 0 | 1.777615 | 98,779,136 |
| 002 | 60.005986 | 60.018200 | 0 / 0 | 1.975810 | 100,319,232 |
| 003 | 60.014229 | 60.009247 | 0 / 0 | 1.798103 | 97,959,936 |

CPU and RSS cover the whole child process, including startup, settling and the
qualification sampler; these are not isolated idle CPU measurements. Zero draws
does not mean no runtime wakeups: the Bonsai clock/runtime still polls under the
accepted clock policy. No FPS or physical display-presentation claim is made.

The native probe samples AppKit visibility and activation at absolute one-second
deadlines; AX independently samples foreground state. Each phase has 60 or 61
native observations and 60 AX observations, all with expected state and bounded
gaps. This establishes sampled visibility, not continuous exposure between
samples. The collector gives foreground to the existing Finder for the unfocused
phase without changing its windows or files. It checks actual editor blur; a
focused blinking caret is deliberately outside settled-idle acceptance.

Raw reports and matching complete application records:

- [Run 001](idle-run-001-och17.json), [application log](idle-run-001-och17.log).
- [Run 002](idle-run-002-och17.json), [application log](idle-run-002-och17.log).
- [Run 003](idle-run-003-och17.json), [application log](idle-run-003-och17.log).

Reproduction at the stated revision:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_idle/main.exe
python3 scripts/test_measure_idle.py
python3 scripts/measure_idle.py --build-profile release --smoke --output scratch/idle-warmup-001 --timeout 180
python3 scripts/measure_idle.py --build-profile release --output scratch/idle-full-001 --timeout 180
```

Repeat the separate warmup/full pair three times. These runs used `--executable`
pointing at a preserved copy of that release executable, verified by the hash
above. The owned `caffeinate -di` helper and all test children were reaped after
the batch. The raw logs were revalidated with the committed collector before
copying this evidence.

Earlier smoke preflights 001–003 failed before timing because an attempted
CoreGraphics bounding-rectangle exposure check treated the Dock's mostly
transparent fullscreen rectangle as opaque. That approximation was removed;
the native AppKit occlusion flag now matches the display-link gate. Subsequent
smokes 004/005 and the three final warmups passed. Failed preflights remain in
local scratch; they are not passing measurements.

Validation also includes 928 native tests (two existing skips), strict native
Clippy, paired OCaml/Rust probe checks, formatting and portable collector tests.
Probe v3 sampling is qualification-only and stops at Finish/unmount; ordinary
Begin does not start it. Linux OS visibility remains unknown and is rejected by
this macOS collector. Linux compilation is not graphical acceptance. Remaining
native-entity/GPU/presentation, accessibility, distribution and other release
gates remain separate in [status](../status.md).
