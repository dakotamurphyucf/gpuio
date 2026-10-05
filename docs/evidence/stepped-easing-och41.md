# Stepped easing — OCH-41

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
