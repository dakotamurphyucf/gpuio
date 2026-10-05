# Stepped easing contract — OCH-41

The pinned [motion easing source](../catalog/sources/base-motion-easing.rs.txt)
provides four step positions. GPUIO exposes them as
`Animation.Easing.Step_position.{Jump_start,Jump_end,Jump_none,Jump_both}` and
`Animation.Easing.steps ~count ~position`, returning `Or_error.t`.

Counts are integers from 1 through 4,294,967,295, matching the source's positive
32-bit range; `Jump_none` requires at least 2. Sampling uses constant memory and
time, so the count does not allocate a sequence of stages. The public constructor
and Rust decoder both validate these bounds. Application code cannot construct
an invalid abstract easing value.

For clamped progress `p` and count `n`, the output is
`clamp(floor(p*n) + offset, 0, jumps) / jumps`:

| Position | Jumps | Offset | Output at progress zero |
| --- | ---: | ---: | ---: |
| Jump_start | n | 1 | 1/n |
| Jump_end | n | 0 | 0 |
| Jump_none | n-1 | 0 | 0 |
| Jump_both | n+1 | 1 | 1/(n+1) |

All reach 1 at progress 1. Boundary samples select the step after the jump.
Unlike continuous presets, Jump_start and Jump_both jump immediately at zero;
generic endpoint shortcuts must not erase that behavior. Existing native delay,
paint-confirmed completion, repetition, interruption and reduced-motion rules
remain in force. A delayed run holds its initial value until its start; a finite
completed run settles exactly at its declared target.

The paired unpublished epoch-3 representation appends easing tag 7 with an
`int64` count and position tag 0/1/2/3 in the order above. Earlier easing tags
remain unchanged. Both runtimes must be rebuilt together. No new polling,
OCaml frame callback or dependency is introduced.

Validation must cover independent wire fixtures on both sides, invalid counts
and position tags, values immediately around jumps and both endpoints, native
delay/completion behavior, and the public gallery example. Pure/native-state
tests and builds do not establish physical gallery or platform acceptance.
This contract does not add linear-stop curves or change other catalog gaps.
