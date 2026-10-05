# Explicit iteration counts and playback direction

The new `Animation.Repeat.Finite (count, direction)` and `Infinite direction`
policies apply to both `Config.create` and `Program.create`. Existing `Once`,
`Loop` and `Alternate` keep their wire tags and established behavior.

`Iteration_count` is abstract and spans 0 through 18,446,744,073,709,551,615.
Use `of_int` or nonnegative `of_int64` for ordinary counts; `of_string` accepts
decimal digits across the full unsigned range, including leading zeros.
`to_string` produces canonical decimal. Counts are never stored as floating point.

```ocaml
let count = Animation.Iteration_count.of_int 3 |> Or_error.ok_exn in
let repeat = Animation.Repeat.Finite (count, Alternate_reverse) in
Animation.Program.create ~initial ~repeat stages
```

## Timing and direction

An iteration is one complete declared cycle, including stage pauses. Initial
delay applies once outside the cycles: positive values hold the directed starting
endpoint, and negative values advance active elapsed time. `Normal` traverses
forward; `Reverse` traverses backward; `Alternate` starts forward and changes
direction each cycle; `Alternate_reverse` starts backward. Direction applies to
timeline time before easing. A single reverse tween samples `easing(1-progress)`,
including its active progress-1 boundary, which may differ from the target with
non-normalized linear-stop curves.

In advanced programs, explicit reverse traverses the whole compiled timeline,
including stage order and pauses. Springs sample the existing analytic trajectory
backward and negate its velocity. This differs from legacy `Repeat.Alternate`
and `Program.reverse`, which reverse declared intervals while retaining their
easing and stage-delay association. Those legacy operations remain available.

All explicit policies require initial values. Finite counts permit zero-duration
cycles; infinite policies require a positive declared cycle. A zero count plays
no cycle and settles after the initial delay. The terminal direction uses
iteration `max(count-1, 0)`, including zero counts. Finite completion forces the
exact directed endpoint, preserving GPUIO's endpoint contract even when an easing
curve has non-normalized endpoints. A zero-duration finite cycle uses that same
terminal rule.

Retargeting uses the last painted position/velocity for the first compiled cycle;
subsequent cycles use the declared range. Explicit reverse can intentionally jump
to the directed starting endpoint. `Program.restart` uses declared initial values.
`Program.with_repeat` changes the policy while retaining playback and the restart
token and revalidates initial values, duration, shared-clock and encoding bounds.

## Events, suspension and clocks

Explicit finite policies deliver exactly one `Finished` after an accepted paint,
without per-stage or per-cycle observations. Replacing, removing or cancelling an
unfinished run retains the existing cancellation contract. Default `Once` retains
its ordered stage observations. Explicit infinite policies never finish.

Paused/hidden runs hold the painted value and exclude suspended time. Restart
resets elapsed time and reapplies the initial offset. Reduced motion settles a
finite policy at its directed terminal endpoint; infinite policies hold their
directed starting endpoint without animation wake requests. Paint/generation
fences continue to reject stale prepared samples and callbacks.

Shared clocks admit explicit infinite policies, require timed stages and zero
initial delay, and compare direction as part of their schedule identity. Joining
or retargeting a shared member uses the declared range at the group's current
phase. Finite policies are independent; a finite member does not silently acquire
an ambiguous late-join completion rule.

## Transport, storage and deadlines

Existing repeat tags 0–2 remain unchanged. Tag 3 carries finite count high and low
32-bit limbs (each encoded as a bin_prot int64), followed by direction tag 0–3 in
the order above. Tag 4 carries infinite direction. Each limb is independently
validated before constructing native state. Paired packages must be upgraded
together; older decoders reject the new variants. Existing configuration byte
limits include the added payload.

Counts do not expand arrays, stages or events. Native evaluation uses bounded
first/subsequent compiled tracks and integer division/modulo. Cycle products and
completion comparisons use u128 nanoseconds. OS timer waits are bounded to one
day to avoid overflowing native clock/timer APIs; longer waits recheck at that
horizon, without per-frame polling. A bounded wake does not certify completion
of an unelapsed count. Constant finite motion waits for completion, constant infinite
motion stays idle, and reverse stage pauses schedule boundary wakes rather than
requesting frames throughout the pause.

Tests and physical qualification are separate from this contract. The Motion
gallery demonstrates finite directions, zero cycles and continuous reverse with
pause/resume/cancel controls. Current acceptance is tracked in OCH-41/OCH-17.
