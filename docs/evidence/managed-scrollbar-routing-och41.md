# Managed collection scrollbar routing — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max, based on `beaa6244`.
This repair covers public Bonsai adapter routing and scoped native input;
OCH-41/OCH-17 remain open.

## Defect and API repair

The gallery applied `View.with_scrollbar (Output.view output)` after constructing
managed components. Lists, trees and selectable lists return layout wrappers;
the native viewport is inside them. The decoration therefore stayed dormant on
an outer container without overflow. The first physical test failed to find the
message list's custom range despite enabled presentation controls. The saved
`root-001` screenshot shows that missing bar.

`Gpuio_bonsai.Virtual_list`, `Tree_rows`, `Tree`, `Selectable_list` and `Table`
now accept `?scrollbar:Gpuio.Scrollbar.t option Bonsai.Cont.t`. List reactive and
paged variants and the paged table forward it too. It defaults to `B.return None`.
The adapters apply the description to the actual native viewport before wrapping
it, preserving existing row/controller lifetimes. `None` clears custom
presentation; the configuration's scrollbar enable flag remains authoritative.
Table already returned its native root, but now offers the same argument.
`View.with_scrollbar` retains its single-node semantics. No protocol, native
renderer, dependency or toolchain change is needed.

Collections derives the description once and passes it to the message list,
outline, table, horizontal cards and searchable list. Their returned output views
are no longer decorated at the wrong level. Adjacent walkthroughs and the
[design contract](../design/scrollbar-presentation.md) explain that distinction.

## Deterministic and native checks

The new `scrollbar_component_test.ml` verifies all five component families:
metadata reaches a native owner and no outer wrapper; changing, clearing and
reapplying it preserves row keys and activation/deactivation counts. The full
OCaml suite, formatting and gallery build pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

`--section managed-scrollbars` exercises message list, outline and result table
across Light/Dark × Comfortable/Large/Compact (18 cases per executable). It checks
matching vertical arrows, Home/End, AX Increment/Decrement, terminal-row presence,
retained offsets across gradient/motion changes and custom-presentation removal
and restoration, and absent ranges after page retirement. The outline expands
Archive to create actual overflow before asking for a bar.

Two intermediate driver attempts (`root-002` and `root-003`) failed because the
shared AX helpers deliberately prune table/outline descendants by default. The
helper APIs now permit explicit `search_files=True`, used for these nested
ranges and tree rows, including absence checks. Defaults for other callers are
unchanged. This was test setup; neither attempt is reported as passing.

The repository matrix (`root-004`) and fresh installed consumer
(`installed-001`) each pass all 18 cases. The independent build also passes
both extension catalog checks. Both test apps close and are reaped.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20261007-access-check/managed-scrollbar-consumer
python3 scripts/test_gallery.py --section managed-scrollbars --images <report-directory>
python3 scripts/test_gallery.py --section managed-scrollbars \
  --executable <consumer>/_build/default/main.exe --images <report-directory>
```

Both local driver invocations use a 360-second watchdog; the driver closes/reaps
its app in cleanup. The required macOS CI matrix adds a six-minute bounded step.
Example inventory (432 source files/268 groups), catalog audit, Python compilation,
focused Ruff, no-new-findings comparison against the large driver's 244 existing
Ruff findings, workflow actionlint and whitespace checks pass.

## Evidence and limits

The [verified archive](managed-scrollbar-routing-och41/reports.tar.gz) and
[manifest](managed-scrollbar-routing-och41/manifest.json) retain source snapshots,
failed and passing run logs, screenshots, reports, build and audit output.

Repository executable SHA-256:
`d3446230e31f4e3ee55f3c41a17b6edc35df839406cd7d3827152962416be055`.
Installed executable SHA-256:
`1952f7e71feefa5901189d6eb254f5c3c930608986fa6ed521cb9bb0db140436`.

This matrix checks vertical range behavior for three managed owners. Horizontal
cards and selectable-list routing have deterministic coverage here; their full
scrollbar gesture matrices remain separate. This is not pointer-drag, hardware
trackpad, VoiceOver speech, per-frame timing, resource or Linux desktop acceptance.
It does not establish the cause or resolution of the separately reported list
jitter. The owner confirmed that changing windows may have interrupted an earlier
benchmark; that attempt is inconclusive. The hosted follow-up below now validates the repository interaction step;
whole-candidate integration remains pending.

## Hosted repair validation — 37825342234

At `1f360479`, the macOS job's **managed collection scrollbar input and retained
offsets** step passes, as do the OCaml/Rust suite and independent public gallery
consumer build. The [focused step snapshot](milestone-07-ci/run-37825342234-managed-scrollbars.json)
records their actual conclusions from
[the workflow](https://github.com/dakotamurphyucf/gpuio/actions/runs/37825342234).
The interaction command is:

```sh
python3 scripts/test_gallery.py --section managed-scrollbars \
  --images .cache/ci/managed-scrollbars
```

This invocation exercises the repository gallery. The hosted independent consumer
step is build-only; the installed interaction matrix remains the separately
recorded local result above. The macOS job later finished with a separate
dates/colors failure; this closes the repair's hosted follow-up without claiming complete candidate CI,
full gallery acceptance, VoiceOver or physical frame-timing qualification.
