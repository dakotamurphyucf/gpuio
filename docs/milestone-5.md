# Milestone 5 implementation and handoff

The accepted milestone scope is implemented with local macOS acceptance.
[PR #13](https://github.com/dakotamurphyucf/gpuio/pull/13) records the required
hosted macOS/Linux checks, checked source revision and merge state. Ticket closure
requires both hosted jobs and merge; local acceptance alone is insufficient.
The release-level Linux GUI gate remains OCH-17. This handoff complements the
[complete component-to-chat matrix](design/agent-chat-m5-showcase.md).

## Start here

Read `AGENTS.md`, [engineering standards](design/engineering-standards.md),
[development setup](development.md) and the relevant live Linear ticket. Use the
checkout's isolated stock OCaml 5.3/Bonsai 0.17/Core/Eio and pinned Rust/GPUI
toolchains. Do not change another repository's switch or global defaults. Keep a
personal ticket notepad under ignored `scratch/agents/<session>/` and preserve
accepted contracts/evidence in versioned docs and Linear.

`lib/core/*.mli` contains public domain contracts; `lib/bonsai` contains presenters
and typed effects; `lib/eio` owns asynchronous resources, producers and scopes.
`lib/protocol` and `rust/protocol` define paired encodings. `rust/native` owns GPUI
state and interaction; `rust/extension-sdk` is the package-author integration
surface; `rust/table` is the provenanced table adapter. Read vendored provenance
before modifying extracted upstream code. Both bridge halves currently negotiate
mask `2199023255551`; individual capability bits are not independently deployable
wire compatibility promises.

## Delivered families

| Ticket | Public capability and native ownership | Contracts and evidence |
| --- | --- | --- |
| OCH-23 | Typed packaged native components; properties, sequenced commands, revocable asynchronous events, generated single native backend | [SDK](design/extensions.md), [acceptance](evidence/extensions-och23.md) |
| OCH-24 | Immutable typed canvas scenes/resources, scoped revisioned uploads, native drawing/hit testing/drag/pan/zoom and accessible objects | [Canvas](design/canvas.md), [acceptance](evidence/canvas-och24.md) |
| OCH-25 | Physical springs, ordered sequences, application/group clocks, retargeting/pause/cancel/reduced motion and native scheduling | [Programs](design/animation-programs.md), [acceptance](evidence/animation-programs-och25.md) |
| OCH-26 | Native container-size breakpoints with retained branches, explicit hidden-input/focus rules and typed painted-selection observations | [Queries](design/container-queries.md), [acceptance](evidence/container-queries-och26.md) |
| OCH-33 | Presentation/cards/forms, semantic metadata, avatars/fallback, rating, skeleton/shimmer/spinner and feedback | [Components](design/presentation-components.md), [acceptance](evidence/presentation-components-och33.md) |
| OCH-34 | Single/range sliders, numeric editor/stepper variants and segmented OTP, native drafts/IME/clipboard, typed commands and observations | [Numeric](design/numeric-inputs.md), [OTP](design/otp-inputs.md), [acceptance](evidence/numeric-inputs-och34.md) |
| OCH-35 | Inline calendar and popup date selection, inclusive civil-date ranges, constrained navigation, draft/apply/cancel | [Dates](design/calendar.md), [acceptance](evidence/calendar-och35.md) |
| OCH-36 | Concrete RGBA/HSLA/hex colors, swatches/channel controls and controlled popup selection with isolated drafts | [Colors](design/color-inputs.md), [acceptance](evidence/color-inputs-och36.md) |
| OCH-37 | Disclosure/accordion, navigation history/breadcrumbs/sidebar/pagination, sheets/alerts/hover cards and native carousel behavior | [Navigation](design/navigation-components.md), [acceptance](evidence/navigation-components-och37.md) |
| OCH-38 | Stable keyed managed trees, expansion/selection/reveal/typeahead, lazy Eio loading and application-approved move proposals | [Trees](design/managed-trees.md), [acceptance](evidence/managed-trees-och38.md) |
| OCH-39 | Read-only virtual tables, stable row/cell identity, native columns/selection/copy/context, application sorting/filtering and bounded Eio paging | [Tables](design/data-tables.md), [audit](evidence/data-tables-och39-audit.md) |
| OCH-46 | Purposeful integration of every family into the polished agent workspace | [Coverage matrix](design/agent-chat-m5-showcase.md), [evidence](evidence/agent-chat-m5.md) |

## Contracts that must survive future changes

Rust owns synchronous layout, paint and immediate native input. OCaml owns
application data and receives bounded queued observations. Do not add synchronous
OCaml render/delegate/easing callbacks or expose borrowed native pointers. Public
IDs, revisions, query generations and mounted leases serve distinct purposes;
validate all applicable identities before accepting delayed results or actions.

Native editors own draft text, directional selection, marked composition and undo
history. Ordinary reactive updates must not replace their current contents.
Commands have explicit revision/replacement semantics. Row eviction, hidden
native content, Bonsai deactivation and Eio scope cancellation are separate events;
conversation-owned streaming survives virtual row eviction. Resource statistics
measure documented reservations and live owners, not total process/GPU memory.

The SDK is trusted statically compiled code, without a dynamic loader or stable
binary ABI. Its initial caps are 64 names, 256 instances/window, 64 KiB properties
and 16 KiB command/event payloads. Custom native callbacks/layout/unsafe code
remain the package author's responsibility; host hook containment is not a sandbox.

Tables are read-only, with at most 64 columns and explicitly bounded active cells.
Sorting/filtering belongs to the application or producer; sorting a fetched page
does not sort a remote query. Tree moves are proposals requiring application
approval. Civil dates do not imply time zones or OS locale inference. Native
animation runs without per-frame OCaml transactions, but the Bonsai clock still
uses the accepted periodic runtime wake strategy.

## Run and validate

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/agent_chat/main.exe
_build/default/examples/agent_chat/main.exe
```

Explore workspace exposes Sources, Results, Diagram and Review. Settings provides
generation controls plus explicitly simulated connection/scheduling flows. The
tour links to the same surfaces. The demo uses deterministic local data, including
optional 100,000-item fixtures; it is not an LLM provider client or persistence
service. See its [README](../examples/agent_chat/README.md) for inputs and limits.

Consolidated local checks pass: Dune `@all @runtest @fmt`, Rust workspace tests,
workspace/native/table Clippy, Rustfmt, native executables, all 15 chat walkthroughs
and `scripts/test_extension_consumer.py --run` in a fresh staged workspace. Run
GUI checks sequentially; each must close/reap its owned app. The table-history
wrapper intentionally tests failure cleanup as well as two complete 100,000-row
traversals; its documented timeout is longer than ordinary native smoke tests.
Exact hosted gates are in [Foundation](../.github/workflows/foundation.yml).

The combined four-window scenario measures actual keyboard input during streaming
with tree/table/canvas/extension work, painted native animation and repeated scoped
cleanup. Its latency is key-post to accessibility readback, not input-to-pixel;
RSS samples are not peak/GPU-cache guarantees. Full/reduced-motion and light/dark,
responsive settings, overlay/focus and screenshot checks have separate evidence.

Two user-reported jitter regressions are covered: retained transcript rows no
longer churn during wheel scrolling, and asynchronous Markdown/code preparation
no longer inserts/removes normal-flow loading content that shifts earlier cards.
Installed document content remains visible during preparation; initial status
occupies the existing toolbar. Real content growth and edits may still reflow.

## Next milestone and release boundary

Milestone 6 contains OCH-27 deep links/document integration, OCH-28 OS notifications,
OCH-29 broader graphics/independent-package examples and OCH-40 reusable charts.
Do not treat their services as present in this milestone's local simulations.
OCH-17 and OCH-41 own expanded-v1 platform/distribution validation and the final
catalog/gallery ledger. Full code editing/LSP, editable grids, docking and other
post-v1 subsystems retain their existing later tickets. Windows, dynamic plugins
and hot reload are not required here.
