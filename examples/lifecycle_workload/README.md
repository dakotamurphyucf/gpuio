# Shared resource lifecycle workload

This library opens, exercises and retires one window at a time for the
[performance lifecycle driver](../performance_lifecycle/README.md) and the
[resource audit driver](../resource_audit/README.md). It is not a standalone executable.

Read the [code walkthrough](gpuio_lifecycle_workload.md) for resource ownership,
Bonsai rendering, Eio coordination, collector acknowledgements, exact commands
and the limits of the diagnostic assertions.
