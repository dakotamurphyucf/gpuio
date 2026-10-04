# Inherited output readiness — OCH-17

Local macOS arm64, 2026-10-03, dirty worktree based on `83eb87e`. This fixes the
reference applications' redirected-output failure found during local packaging;
it does not establish desktop, clean-machine or full release acceptance.

## Reproduced boundary

The pinned Eio Posix backend polls blocking output before writing. Local macOS
`poll(POLLOUT)` returns `POLLNVAL` for a valid `/dev/null` descriptor, while `fstat`
succeeds and `select` reports it writable. Eio's scheduler asserts that POLLNVAL
cannot occur. Regular files and pipes return POLLOUT normally. The raw Eio probe
still fails after this change, confirming no switch/package source was modified.

`Gpuio_eio.Output.write sink text` is a narrow descriptor adapter. Generic sinks
use normal `Eio.Flow` output. Unix descriptor classification runs in Eio's supported
system-thread pool; blocking character devices perform the write there without
polling. Files, pipes, sockets and nonblocking descriptors retain normal Eio flow
handling. The supplied Eio descriptor stays borrowed/pinned through each worker
operation with `Eio_unix.Fd.use_exn`; no ambient paths, process-global stream
replacement, descriptor-mode change or early close is used.

Worker writes handle interruption/partial progress and report zero progress as an
I/O error. Unix failures retain typed Eio backend error data. Cancellation is
checked before admission and after the protected worker returns; an in-flight
blocking kernel operation is not interrupted by closing or reusing its fd.
This keeps unrelated fibers runnable but does not promise cancellation can abort
an arbitrary device's kernel write. Call this API from an Eio fiber, never a native
synchronous layout/input callback. It is not a replacement Unix filesystem layer.

Agent Workspace stdout (including diagnostics), gallery/Signal Studio metadata,
and desktop/notification metadata now use the public adapter. Other application
code calling stock `Eio.Flow` directly is not transparently patched.

## Validation

- Eio expect test: generic buffer sink, exact Unicode/NUL bytes, empty and repeated
  writes, and cancellation before output leaves the buffer unchanged.
- `test/output/probe.exe` plus `scripts/test_output_sink.py`: exact 2 MiB pipe and
  regular-file output; `/dev/null` success; read-only descriptor rejected by typed
  `Eio_unix.Unix_error (EBADF, ...)`; no caller ownership or blocking-mode change.
- A private pseudo-terminal initially stays undrained. An independent Eio timer
  must fire while the device write is stalled, then all 2 MiB must arrive exactly
  after draining. This verifies scheduler responsiveness without a GUI window.
- A full nonblocking Eio pipe cancels under backpressure. This does not make a
  stronger cancellation claim for arbitrary inherited blocking pipes.
- All five changed example exporters build. Agent Workspace, gallery, Signal
  Studio and Desktop Lab export valid metadata through pipes and to `/dev/null`;
  Notification Lab also succeeds with `/dev/null` output.

The first probe build corrected a test callback to return the Result expected by
`Eio.Time.with_timeout`. A subsequent error-text assertion was replaced with a
proper typed Unix-backend check; no production failure was suppressed. The final
subprocess driver passes. No AppKit window, AX action, source/switch patch or
system-wide descriptor setting was used. Portable CI includes the same no-display
probe; hosted Linux/macOS execution has not yet been established for this change.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @lib/eio/runtest test/output/probe.exe
python3 scripts/test_output_sink.py
```

The full OCaml test suite, formatting and all five example builds pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt examples/agent_chat/main.exe examples/gallery/main.exe examples/signal_studio/main.exe examples/desktop/main.exe examples/notification/main.exe
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-output-gallery-20261003
```

The fresh independent installed-gallery build reports
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. The temporary
workspace is validation evidence, not a build dependency.

A fresh Agent Workspace artifact was assembled with the existing local packager
and `--sign ad-hoc`, extracted into a new directory, and verified with
`codesign --verify --strict`. From `/private/tmp`, both captured-pipe and
`/dev/null` metadata exports pass with identical application metadata. This
reproduces and closes the exact extracted-bundle failure from the previous
checkpoint. No GUI, Launch Services registration or clean-machine test occurred.

- Archive SHA-256: `465e7363f442fd4dc3ceb314df04f7933ba8ef413d6be826db471bd5a46f40f7`.
- Packaged executable SHA-256: `43140b0d1c9cee2da3288f5de94d9f5b14999aae3c509575a958a1f1c716867d`.

The artifact still includes a review-pending notice set and is not approved for
external distribution. Earlier archived gallery/Signal bundles predate this fix;
their rebuilt source exporters pass, but those old archives are not relabeled. This addresses the output assertion, not the
unrelated historical chart latency sample or the remaining release gates.
