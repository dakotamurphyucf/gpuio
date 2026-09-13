# Runtime implementation boundary

`gpuio.eio` owns the app runner and all first-party asynchronous work. Ordinary
components use `Gpuio_bonsai.View` and Bonsai.Cont; no separate clock API is needed.
`gpuio.runtime_core` is a runner/testing adapter, with checked UI-domain ownership.

A display cycle here means an accepted native tree, not physical presentation.
After an accepted update, lifecycle effects run; subsequent processing flushes
any generated actions. Empty diffs also refresh callbacks and run lifecycles.
There is one additional flush after an empty-diff lifecycle pass, so its actions
can update the view immediately. Further lifecycle turns run on the next wakeup;
unconditional after-display effects cannot create an unbounded synchronous loop.

The shared monotonic Eio timer starts at 60 Hz. Real input and task completions
wake the loop directly. Wall-clock samples advance each Bonsai clock monotonically
(backward wall-clock adjustments clamp until time catches up). There is no idle
GPU frame request. Background windows retain their timer semantics. These rules
supersede older research requirements forbidding periodic idle OCaml updates.
