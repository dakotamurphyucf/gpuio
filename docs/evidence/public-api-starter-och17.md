# Public application entry point — OCH-17

Checkpoint 2026-10-04 on macOS arm64, dirty worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This completes an application-author
entry-point and installed-default-backend check, not the full release/API audit.
OCH-17 and OCH-41 remain open.

## Review and changes

Reviewed actual `Gpuio_eio.App`/`Scope` interfaces, public Dune libraries,
Core editor bounds, bridge version/capabilities, native virtual backend selection,
component SDK compatibility and the current platform/distribution policy.

- Added a compiled [starter application](../../examples/getting_started/main.ml)
  with a public-library-only Dune stanza and ordinary Bonsai state/effects.
- Added [application setup](../getting-started.md) and
  [API compatibility/limits](../api-compatibility.md). These distinguish current
  interfaces from imported pseudocode, document runtime ownership and units,
  and require OCaml/native/package artifacts from the same checkout. An epoch-3
  handshake alone does not prove interoperability between experimental builds.
- Corrected README platform wording to the accepted macOS-first release policy,
  replaced stale claims that the public runner was still being built, and fixed
  an extension example's contradictory statement about smoke-window activation.
- Extended the isolated consumer runner with `--example getting_started`, using
  installed public libraries and the default native backend without composition.
  Its `--run` rejects before workspace creation because this interactive sample
  has no automated GUI driver. Existing extension/gallery/Signal options remain.
- Added a required default-backend consumer build to both macOS and Linux CI
  matrix entries. No hosted execution is claimed by editing that workflow.

## Validation

Logs/workspaces are in ignored `scratch/agents/root-20261003-release-notices/`.
No OS windows were opened, and no opam switch/default or dependency selection
was changed.

- `GPUIO_JOBS=2 ./scripts/gpuio build examples/getting_started/main.exe` passed:
  `starter-build-001.log`.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example getting_started --workspace scratch/agents/root-20261003-release-notices/starter-consumer-001`
  passed with `run=False`: `starter-consumer-001.log`. The separate consumer's
  Dune build resolves staged public OCaml libraries and the installed default
  native archive. It does not need an extension manifest or generated backend.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example extension_consumer --workspace scratch/agents/root-20261003-release-notices/starter-extension-regression-001`
  passed with `run=False`: `starter-extension-regression-001.log`. This also
  builds the copied document-profile package's codec tests and generated native
  backend with the existing reviewed Cargo lock.
- Python syntax, rejected interactive `--run`, eight edited-document relative
  links and YAML parsing/unique step IDs/required two-platform placement pass.
  The local Ruby version lacks `filter_map`; the one-off YAML check used
  `map.compact` instead. No dependency was installed for this check.
- Jane Street formatting was applied to the starter, without expect promotions.
  `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` passed
  (`starter-format-check-001.log`), as did `git diff --check`.

## Limits

A staged consumer on the development machine is not a clean-machine source or
packaged GUI test. The starter has not received a fresh physical input/visual
walkthrough. This checkpoint does not rerun every native test, qualify Linux GUI,
prove stable API compatibility, choose the first release version, or publish an
opam/binary package. Full catalog/physical/performance/distribution/hosted/review
and publication requirements remain in [status](../status.md).
