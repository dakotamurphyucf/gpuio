# Example walkthrough coverage — OCH-48

2026-10-06, source base `f73a8f9`, macOS 14.5 arm64. This checkpoint starts the
explicit documentation inventory; it does not complete OCH-48 or milestone 07.

The [inventory](../../examples/coverage.md) covers **417 OCaml/Rust source files
in 260 groups**, with each `.ml`/`.mli` implementation/interface pair owned
together and Rust sources classified separately. All groups remain visible,
including historical bootstrap, generated registration, extension-author,
benchmark, diagnostic and test support. Existing README presence is not treated
as a completed review. Build metadata and generated-output handling are explained
in the [review guide](../../examples/coverage-guide.md).

Four groups have been reviewed against their actual source and OCH-48's content
requirements:

- [Getting started](../../examples/getting_started/README.md): integer model,
  pure view versus Bonsai state, event/effect/update trace, startup/closure and
  a concrete decrement adaptation.
- [Embedded palette](../../examples/gallery/embedded_palette_preview.md): stable
  commands, persistent native query, model actions, native editor target,
  hide versus unmount and focus-command limitations.
- [External palette](../../examples/gallery/external_palette_preview.md): mock
  search, serial/query identity, Eio cancellation, accepted-view staging followed
  by guarded result publication, errors and bounded-result adaptation.
- [Preview scope](../../examples/gallery/preview_scope.md): activation acquisition,
  partial-failure cleanup, late-result suppression and application/page ownership.

The other **256 groups remain pending review**, including existing companions
that may already satisfy much of the checklist. The source index makes these
omissions reviewable; it is not a count of missing implementations.

`scripts/audit_example_docs.py` passes source ownership, path and generated-table
checks. It deliberately allows explicitly pending rows. It does not infer prose
quality from links or mark reviews automatically. A new required foundation step
runs this same check on macOS and Linux; hosted execution of that step is pending.
Contributor and agent instructions now require companion maintenance.

Local checks pass:

```sh
python3 scripts/audit_example_docs.py --write
python3 scripts/audit_example_docs.py
python3 -m py_compile scripts/audit_example_docs.py
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio build examples/getting_started/main.exe
git diff --check
```

Relative file links in changed Markdown were checked for existing destinations.
This does not validate external URLs or every heading anchor. The gallery and
starter documentation use their existing launchers; no GUI test was repeated for
this prose/inventory-only change. Previous [starter/gallery behavior evidence](example-readability-och17.md)
and the scoped [embedded](palette-embedded-och41.md) /
[external](palette-external-results-och41.md) palette checks retain their original
source/platform limits. No new GUI, IME, VoiceOver, performance or Linux desktop
acceptance is claimed.
