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

## Gallery application structure follow-up

Starting from `a370318`, five adjacent guides now explain six more source groups:
[`main`/`Application`](../../examples/gallery/application.md),
[`Component`](../../examples/gallery/component.md),
[`Shell`](../../examples/gallery/shell.md),
[`Pages`](../../examples/gallery/pages.md) and
[`Palette`](../../examples/gallery/palette.md). The application guide owns both
entry-point dispatch and startup; neither source part is omitted. The other
guides link their interfaces and supporting model sources without marking those
models' separate coverage rows complete.

The review covers initial/per-window/shared state, explicit Eio services, the
four-window demo bound and cleanup, native observations, theme synchronization,
pure layout/effect inputs, native-handshake capability gating, stable page keys,
branch deactivation and the local Presentation/Controls/Text editing examples.
It distinguishes Bonsai model retention from native resource retirement and
documents mock operations and discarded command results. Each guide gives a
concrete adaptation. README reading maps link all five companions.

Coverage is now **10 reviewed groups and 250 pending** out of the same 260;
the 417-file source inventory is unchanged. The earlier four-group checkpoint
above remains historical.

The source/table audit, gallery build and changed-Markdown file-link checks pass.
These documented non-GUI modes were also executed successfully:

```sh
./scripts/gpuio exec _build/default/examples/gallery/main.exe --check-catalogs
./scripts/gpuio exec _build/default/examples/gallery/main.exe --print-info-plist
```

The former reports `GALLERY_CATALOGS_PASS counter=1 document_profile=1`;
the latter's output passes `plutil -lint`. No native window was opened for this
documentation-only follow-up. The ongoing PR run `37447717604` covers `548bcde`,
not these newer docs/CI changes, so it cannot certify their new audit step.

## Theme loading and stale-result ownership follow-up

Starting from `8ba99e2`, the adjacent
[theme preview](../../examples/gallery/theme_preview.md),
[appearance model](../../examples/gallery/model/appearance.md),
[selection identity](../../examples/gallery/model/theme_selection.md),
[profile decoder](../../examples/gallery/model/theme_profile.md) and
[file adapter/test](../../examples/gallery/files/theme_file.md) guides explain
six additional source groups. The adapter guide explicitly owns its associated
expect-test source and Dune fixture setup; the interface files are linked too.

The review follows the actual one-request-at-a-time card, picker/read selection
fences, scope cancellation, retained draft and last-good palette. It explains
process-local token identity, the distinction between selected preference and
effective appearance, the 16 KiB/UTF-8/name/parsed-depth constraints, all ten
concrete color fields, bounded Eio reading and cancellation propagation. The
profile guide explicitly places the depth check after S-expression parsing.
These are application example contracts, not an upstream theme-format claim.

Coverage is now **16 reviewed groups and 244 pending**, with all 417 source files
still mapped. README and related guide links expose the complete theme path.
Inventory/table and changed-Markdown file-link checks pass, as does
`git diff --check`. The documented command also succeeds using existing Dune
build/test caching:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @examples/gallery/files/test/runtest
```

No implementation or tests changed. Earlier actual parser/filesystem/native
results retain the scope recorded in [theme-file evidence](gallery-theme-files-och41.md).
No GUI window was opened, and no additional IME, accessibility, performance or
Linux desktop acceptance is claimed by this documentation review.

## Parallel beginner walkthrough review

Starting from `1d623f7`, the owner authorized GPT-6.1 Sol agents to write separate
example guides in parallel. Three agents read their assigned implementation and
public interfaces, kept independent ticket notes and edited only their scoped
documentation. The primary agent reviewed their delivered guides and maintained
the shared source inventory. No agent ran competing builds or GUI tests.

Eighteen additional source groups are reviewed:

- Chart Studio's [application](../../examples/charts/main.md) and
  [index adapter](../../examples/charts/gallery.md), plus six pure sample pairs:
  [catalog](../../examples/charts/samples/gpuio_chart_samples.md),
  [categorical](../../examples/charts/samples/categorical.md),
  [stacked](../../examples/charts/samples/stacked.md),
  [ordinal](../../examples/charts/samples/ordinal_colors.md),
  [inspection](../../examples/charts/samples/inspection.md) and
  [Sankey presentation](../../examples/charts/samples/sankey_presentation.md).
- The [controls](../../examples/controls/main.md),
  [text input](../../examples/text_input/main.md),
  [combobox](../../examples/combobox/main.md) and
  [menus](../../examples/menus/main.md) applications.
- Agent Chat's [CLI](../../examples/agent_chat/main.md),
  [application](../../examples/agent_chat/application.md),
  [fake backend and its separate test group](../../examples/agent_chat/model/fake_backend.md),
  [message composition](../../examples/agent_chat/runtime/chat_message.md) and
  [motion](../../examples/agent_chat/runtime/chat_motion.md).

The guides teach concrete function/type/API names, state construction and
reactive syntax, effect execution, native ownership and representative event
traces. They distinguish synchronous Option syntax from Bonsai syntax, editor
seeds from guarded live commands, native query from application selection, and
conversation-scoped streams from mounted row views. Diagnostic flags and expert
hooks are identified rather than recommended as ordinary application structure.
Each companion is discoverable from its local README and provides an adaptation.

Coverage is **34 reviewed groups and 226 pending**, still covering all 417 source
files. This is documentation review, not a percentage of feature implementation.
Source/table audit, changed-Markdown local file links and `git diff --check` pass.
Agents checked documented commands against source/Dune/driver options; the
primary agent's full `dune build -j2 @all @runtest @fmt` also passes at this source
checkpoint. Native runs performed for the separate rich-label implementation are
recorded in [their own evidence](chart-node-labels-och41.md), not generalized to
these prose changes. Existing recorded platform limits remain in force.

## Scoped page/controller/conversation batch

Starting from `c22a61c`, the same three authorized documentation agents reviewed
eight further source groups: the gallery's [canvas](../../examples/gallery/canvas_page.md),
[assets](../../examples/gallery/assets_page.md),
[documents](../../examples/gallery/documents_page.md) and
[charts](../../examples/gallery/charts_page.md) pages; the positioned-menu
[launcher](../../examples/menu_controller/main.md),
[single-window component](../../examples/menu_controller/component.md) and
[multiwindow component](../../examples/menu_controller/multiwindow.md); and Agent
Chat's [conversation owner](../../examples/agent_chat/runtime/conversation.md).
Nested Phase/Message modules are explained in the conversation guide; separate
files are not invented. The primary agent reviewed the prose and updated the
shared inventory and README discovery links.

Coverage is now **42 reviewed groups and 218 pending**, still 417 source files.
The guides trace actual source publication, native commands and observations,
Bonsai state/effects, activation disposal, stream acceptance/cancellation and
resource ownership. They expose a confusing existing example detail: the Code
tab's append/reset controls currently mutate Markdown. The guide describes that
limitation without inferring design intent; it needs a separate example repair.
No such code repair is claimed by this documentation batch.

Relative local file links, source/table audit and whitespace checks pass. Agents
checked command names/flags against code/Dune/drivers and ran no builds or GUI.
The prior full repository build covers these unchanged implementations; current
Sankey ribbon-color work is separate and not certified by this prose review.
No new platform/input/performance acceptance is claimed.
