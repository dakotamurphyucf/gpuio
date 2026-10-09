# Developer preview installation and API handoff — OCH-17

2026-10-08, macOS 14.5 arm64. This closes R1's source installation,
documentation and experimental API handoff for the developer preview. Final
candidate CI, release-input review and publication remain separate C2/D1/R2
items. No stable API, clean-machine GUI or binary SDK qualification is claimed.

## Independent installation

The documented starter consumer command completed successfully in a fresh
workspace, using the repository's isolated toolchain:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py \
  --example getting_started \
  --workspace scratch/agents/root-20261007-access-check/preview-starter
```

The script builds the public install targets, stages GPUIO and the native Bonsai
packages into a private prefix, and builds a separate Dune project with that
prefix on `OCAMLPATH`. It does not install into or change an opam switch. The
consumer uses only `core`, `gpuio`, `gpuio.bonsai`, `gpuio.eio`, `bonsai` and the
documented PPX dependencies; it requires no application Rust code.

The build observed `87b43c73`. Changes through `1f360479` are documentation and
evidence only. The copied `main.ml` is byte-identical to the current starter.
The development-profile executable SHA-256 is
`ddc967d371619264cbf2917dd6c4abdb48613f8254f697ba9bd3fcfa26b61f5f`.
The [six-file archive](preview-installation-och17/reports.tar.gz) retains the
complete build log and consumer source/configuration; its
[manifest](preview-installation-och17/manifest.json) was independently checked
against the archived bytes. No executable is included in that archive.

This fresh check is **build-only**, not a new GUI walkthrough or fresh-machine
bootstrap. Earlier [installed starter interaction evidence](example-readability-och17.md)
covers Increment, Reset and normal Close at its recorded source checkpoint.
The current [coverage inventory](../catalog/preview-coverage.md) links broader
application/component behavior, and the current candidate's required hosted
bootstrap/consumer/native checks remain R2. Preserve those distinct scopes;
an installed build alone does not prove platform interaction.

## API and onboarding review

Reviewed the adoption path from [development](../development.md),
[getting started](../getting-started.md) and the adjacent
[starter walkthrough](../../examples/getting_started/README.md) against the
actual starter and consumer script. The guide explains `counter_view`, reactive
state/effects and `App.run`, the separate public libraries, native ownership,
stable collection identities and Eio scope placement. It describes an isolated
installed prefix rather than promising an already published opam package.

The [API layer map](../api-layers.md) and
[compatibility guide](../api-compatibility.md) incorporate the existing
[boundary review](api-boundaries-och17.md), including resource admission versus
native publication, paging cancellation/concurrency, task/scope cleanup and
desktop-service completion. The reviewed cleanup and paging defects have their
linked deterministic regressions. This is sufficient for the experimental
preview contract; it is not a claim of an exhaustive audit of every interface.

The [preview guide](../developer-preview.md) gives the starter, gallery, chat and
graphics reading paths, platform/API limits and a GitHub reproduction route.
The bug and API-feedback templates request concrete application cases and small
reproductions. The example-doc inventory covers 432 sources in 268 groups; its
structural check supplements the separate OCH-48 walkthrough review.

Corrected the compatibility guide's stale statement that every signed-app and
full accessibility qualification remained a milestone-07 preview gate. Those
claims remain unqualified and belong to OCH-164; Linux desktop belongs to OCH-47.
The final published tag/commit must be added during R2 publication. This record
does not announce that a preview has already been published.
