# Results column geometry readiness — OCH-17

This checkpoint repairs a test-driver readiness condition. **The full native
Results flow still fails locally and is not accepted.** No runtime behavior,
pointer assertion, release threshold or platform requirement is relaxed.

Hosted run [37558498842](https://github.com/dakotamurphyucf/gpuio/actions/runs/37558498842)
fails reading `RESULT` header geometry after Diagram → Back. The existing driver
waits for a header object, then reads its geometry once. A deterministic fixture
with a published header whose position is not yet available reproduces the old
`Missing geometry: RESULT` failure.

`scripts/test_agent_chat_results.py` now reacquires the current header until it
has finite, positive geometry that remains stable for 100 ms, within the existing
five-second deadline. All temporary AX references are released on every sample.
Persistent missing/invalid/moving geometry still fails; unexpected traversal
exceptions propagate. A slow native call cannot admit coordinates after the
deadline. Data cells with the same column label remain excluded from header lookup.
This is coordinate readiness, not proof that a target is unclipped or clickable.

Validation on macOS 14.5 / Apple M1 Max:

- Running the old `HEAD` driver implementation against the delayed-position
  fixture reproduces `Missing geometry: RESULT`.
- `python3 scripts/test_results_geometry.py` passes five tests, including six
  persistent-failure subcases, reference cleanup, moving geometry, deadline
  overrun and exception propagation. CI runs this portable regression on both
  foundation platforms; a local Linux execution is not claimed.
- `python3 scripts/test_agent_chat_results.py` still fails the first column
  resize with the pre-existing local executable. The header is above the
  inspector's visible page region; finite AX geometry alone is insufficient.
- A scoped diagnostic scrolls the inspector to its top using a native wheel
  event with an explicit screen location. Resize then succeeds. The first cell
  click still fails to select: its center moves from `(817.5, 549.0)` to
  `(817.5, 499.5)` between mouse-down and mouse-up. Screenshots show the whole
  Results page moving; the release is over another row. This is measured
  movement, not yet proof of which focus/scroll path caused it.

The diagnostic deliberately stops after recording down/up behavior; its nonzero
exit is not a passing walkthrough. All owned child applications are terminated
and reaped. No VoiceOver settings were changed. Application rebuild provenance is
not inferred: the [manifest](results-column-readiness-och17/manifest.json) records
the pre-existing executable hash and explicitly limits its scope.
[Logs, screenshots, driver and diagnostic sources](results-column-readiness-och17/reports.tar.gz)
are retained in a verified 11-member archive.

Next: reproduce the moving target in a focused native regression, inspect generic
focus reveal and table/ancestor scrolling, fix the responsible behavior, then
rerun the complete physical Results interaction sequence. Do not replace the
original click with retries or claim the hosted failure fully resolved from the
portable readiness tests alone.
