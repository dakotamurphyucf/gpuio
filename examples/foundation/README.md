# Private foundation experiment

This historical scheduling/FFI experiment remains a diagnostic fixture. It uses
its own numeric-node wire format and native exports; new applications should use
the current public View/Bonsai/Eio APIs.

Read the [main walkthrough](main.md) for graph state, keyed lifecycles, diff/
acknowledgement flow, real file capability, exact commands, and diagnostic limits.
The [native boundary](native.md) and [wire format](wire.md) explain their private
Rust counterparts and how they differ from current production contracts.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio smoke
GPUIO_JOBS=2 ./scripts/gpuio smoke --self-test
GPUIO_JOBS=2 ./scripts/gpuio smoke --two-windows
```

The wrapper builds and changes into this directory so `async-message.txt` is
available. Native graphical setup is required. The self-test uses synthetic
native probes, not real keyboard/IME input; the two-window mode bypasses Bonsai.
See [development setup](../../docs/development.md) and
[platform policy](../../docs/platform-release-policy.md).
