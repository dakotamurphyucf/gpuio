# Stepped and piecewise-linear easing — OCH-41

2026-10-05, macOS 14.5 arm64; uncommitted follow-up to `59e3eac`, including
the preceding piecewise-cubic work. [Exact source and validation archive](stepped-easing-och41/local-validation.tar.gz),
[checksums](stepped-easing-och41/manifest.json).

The public `Animation.Easing.steps` constructor exposes Jump_start, Jump_end,
Jump_none and Jump_both. Counts match the pinned source's positive 32-bit range;
Jump_none requires two or more steps. Rust samples in constant time and preserves
immediate zero-progress jumps before continuous-curve endpoint handling.
The [contract](../design/stepped-easing.md) defines boundaries, delays and paired
unpublished epoch-3 encoding. No OCaml frame callback or dependency is added.

Local checks pass:

- Six animation protocol tests cover independent position-tag fixtures, invalid
  counts, unknown position tags inside transactions, maximum-count decoding,
  and values immediately before, at and after each four-step jump.
- Eleven native motion tests include delayed start, immediate first jump,
  step plateaus, early arrival at the target without premature completion,
  and completion acknowledged after the final paint.
- Public OCaml constructor/fixture expect tests, the gallery build and formatting
  pass. The first check failed only on a missing blank line between Dune stanzas;
  its original log is retained alongside the corrected successful build.
- Targeted strict protocol Clippy, Rust formatting, Python driver compilation
  and structural catalog checks pass.

The gallery adds all four selectable policies. Its physical driver now checks
intermediate discrete widths in both directions; this extended driver has not
been executed. A fresh installed-consumer build for stepped easing also remains
open: the preceding consumer build covers the piecewise-cubic checkpoint only.
These results do not establish Linux desktop, VoiceOver, presentation timing or
complete motion-family acceptance. Linear-stop easing remains unimplemented.

## Installed public gallery qualification — 2026-10-05

At clean implementation `6bc15d3`, a fresh staged-prefix installation builds the
independent gallery and passes both extension catalog checks. Its actual executable
then passes the full native macOS Motion walkthrough, including all three cubic
presets and all four step positions in both directions. Step samples are checked
against their declared discrete widths; exact polynomial and jump-boundary values
remain established by the numeric tests above, not by wall-clock AX sampling.

Existing interruption, sequence pause/resume/cancel/reverse, reduced-motion
endpoints, cross-window policy, shared phase, native keyboard policy action,
page departure/remount and shutdown checks also pass. All owned windows close
and the child is reaped. The captured layout was visually reviewed: both new
control rows fit without overlap; lower content is normally scroll-clipped.

[Build log, walkthrough log, screenshot and exact provenance](stepped-easing-och41/installed-gallery.tar.gz),
[verified checksums](stepped-easing-och41/installed-gallery-manifest.json).
The executable SHA-256 is
`ea5d0efabd6b3fbdb1515010466d799f10d6e06fd766765ccbec8fc809ce1aa2`.
Build session 29309 and walkthrough session 83153 both exit zero. This closes
the installed-consumer and physical walkthrough follow-ups for these easing
additions, superseding the earlier pending statements above. It does not qualify
VoiceOver, physical presentation latency, Linux GUI or the whole motion catalog.

## Piecewise-linear stops — 2026-10-05

The follow-up to `4529746` adds `Animation.Easing.Linear_stop.create` and
`linear_stops`: 2–256 stops with inferred positions, endpoint holds and sharp
duplicate-position jumps. The [contract](../design/animations.md#piecewise-linear-easing)
defines validation, paired encoding, native sampling and variable-size memory
accounting. This supersedes the historical statement above that linear stops
are unimplemented; signed delays and broader iteration/direction gaps remain.

Public OCaml expect tests pass independent wire fixtures, omitted endpoint and
interior positions, coincident inferred positions, malformed values, stop-count
bounds and the full program byte limit. The full root development OCaml suite
passes, followed by the final focused suite and formatting. Native protocol tests
cover both sides of jumps, held endpoints, finite extreme outputs, bounded
untrusted decoding, truncation, program storage and maximum progress/spinner
payloads. Native library tests pass 592 cases with two existing skips, carousel
integration passes five, and motion state tests pass 12, including delayed
linear-stop motion, property clamps and paint-confirmed completion. Strict
native/protocol Clippy passes at that checkpoint.

A fresh independent installation then passes the complete real macOS Motion
walkthrough. It checks held widths, the skipped interval at the duplicate-stop
jump and the final interpolating range in both directions. Existing cubic/step
presets, interruption, pause/resume/cancel/reverse, reduced motion, cross-window
shared phase, page retirement and shutdown also pass. All owned windows close
and the child is reaped. This run did not capture screenshots or qualify
VoiceOver, physical presentation latency or Linux desktop behavior.

Installed executable SHA-256:
`4ae09436a46e0e40ecad24c3f52574b7b1ce643b3aa019d25e7fb8fbcd27c94a`.
The runtime source used by this walkthrough is preserved separately from later
spinner-envelope/accounting and test follow-ups. Those follow-ups do not change
the animation evaluator; the walkthrough is not a spinner behavior qualification.

The broader protocol regression run exposed two stale negative tests: progress
and spinner still treated easing tag 6 as invalid after cubic easing introduced
that tag. Both now test the invalid tag 255. The original failures are retained.
Review also found that maximum spinner/progress labels plus 256 stops require
slightly more than their old 8 KiB standalone envelope. Their envelopes are now
16 KiB, with unchanged 4,096-byte label and 256-stop limits, and spinner retained
memory includes the shared curve. The full protocol suite passes after these
corrections; this changes schema admission limits, not performance thresholds.

Final regression results: all 401 protocol tests pass across 119 suites; the
complete default-feature native test command passes 934 tests across 117 suites,
with two existing skips. Final strict native/protocol Clippy also passes.
[Exact commands, original failures, final logs and source snapshots](stepped-easing-och41/linear-stops/validation.tar.gz)
are preserved with a [verified 48-file manifest](stepped-easing-och41/linear-stops/manifest.json).
The installed walkthrough and numeric tests qualify this addition locally;
current-source hosted checks and the remaining catalog/release work stay open.
