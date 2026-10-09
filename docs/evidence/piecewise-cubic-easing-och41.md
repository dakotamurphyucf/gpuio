# Piecewise cubic easing — OCH-41

2026-10-05, local macOS 14.5 arm64. Uncommitted follow-up to `59e3eac`;
the [source and validation archive](piecewise-cubic-easing-och41/local-validation.tar.gz)
and [manifest](piecewise-cubic-easing-och41/manifest.json) identify exact inputs.

`Animation.Easing.ease_in_out_cubic` evaluates `4*t^3` up to the midpoint,
then `1 - 4*(1-t)^3`. It complements the two previously qualified
[polynomial presets](cubic-easing-och41.md). The CSS `ease_in_out` remains
distinct. Sampling runs entirely in Rust through the existing motion owners.
Paired unpublished epoch-3 easing tag 6 has no payload; earlier tags retain
their bytes. Both runtimes must be rebuilt together.

Local validation passes:

- Public OCaml constructors agree with the shared independent byte fixture;
  the view API expect suite, gallery build and formatter checks pass.
- Five Rust animation protocol tests pass: encoding, transaction decoding,
  endpoints, quarter points, 1,001 polynomial samples and symmetry checks.
- Ten native motion tests pass, including the new exact quarter-point sequence
  and completion acknowledgment only after the final sample is painted.
  These tests exercise the motion state machine without an OS window.
- Strict targeted protocol Clippy, Rust formatting, Python compilation and the
  structural catalog audit pass. Structural coverage does not establish parity.

The Motion gallery and its driver now include the third preset. The new gallery
executable builds. A fresh staged-prefix installation also builds the independent
gallery consumer and passes both extension catalog checks (session 18393, exit 0).
[Consumer logs and provenance](piecewise-cubic-easing-och41/installed-consumer.tar.gz)
and [checksums](piecewise-cubic-easing-och41/installed-consumer-manifest.json)
preserve the exact command and executable hash. The consumer was built, not
executed; the new physical walkthrough remains unperformed. Earlier two-preset screenshots and walkthroughs do not
qualify this addition. No new Linux, VoiceOver or release acceptance is claimed.
Step/linear-stop easing and the other differences in the
[motion review](../catalog/motion-review.md) remain explicit.

Follow-up: the [installed Motion walkthrough](stepped-easing-och41.md#installed-public-gallery-qualification--2026-10-05)
at `6bc15d3` now passes the piecewise preset together with the four step modes.
That newer physical evidence supersedes this checkpoint's unperformed walkthrough;
broader catalog and release acceptance remain separate.
