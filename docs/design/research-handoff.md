<!-- Imported from Linear 0e4d4a56-99f9-4011-8748-28a3680ec6f1 on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

## Accepted expanded v1 scope — 2026-09-10

The user expanded v1 beyond GPUIX component parity. Read [Expanded v1 scope and contracts](<https://linear.app/ochat/document/gpuio-expanded-v1-scope-drawing-extensions-motion-and-desktop-services-46c91a3056cd>). Required: static native component SDK ([OCH-23](<https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk>)), retained custom drawing ([OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>)), springs/sequences/synchronized animation ([OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>)), native declarative container rules ([OCH-26](<https://linear.app/ochat/issue/OCH-26/implement-native-declarative-container-size-breakpoint-rules>)), desktop/deep-link/document integration ([OCH-27](<https://linear.app/ochat/issue/OCH-27/implement-deep-links-and-desktop-applicationdocument-integration>)), OS notifications/actions ([OCH-28](<https://linear.app/ochat/issue/OCH-28/implement-capability-aware-os-notifications-and-action-routing>)) and a broader graphics/extension acceptance example ([OCH-29](<https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently>)).

Preserve milestones 01–04 and the working agent-chat app first. Milestone 05 adds general-purpose UI/extensions; 06 adds desktop integration/examples; 07 is the expanded release gate. Optional platform packages—macOS native tabs/Dock ([OCH-30](<https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration>)), Wayland layer-shell ([OCH-31](<https://linear.app/ochat/issue/OCH-31/optional-add-a-wayland-layer-shell-window-capability-package>)) and native surface investigation ([OCH-32](<https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces>))—are tracked separately and do not block v1. Dynamic plugins/binary ABI, arbitrary synchronous OCaml layout/paint callbacks, full IDE/LSP, terminal, full docking and general multimedia engines remain outside required v1. No deadline was imposed. Earlier canvas/spring deferrals are superseded; historical evidence is unchanged.

## Required OCaml engineering baseline — 2026-09-10

Read the [OCaml engineering standards](<https://linear.app/ochat/document/gpuio-ocaml-engineering-standards-core-eio-and-jane-street-conventions-38f45d0fbed5>) before scaffolding or implementing OCaml code. The user requires Core, Eio for first-party OCaml I/O, Jane Street formatting/PPX and expect-first tests, following Ochat's conventions. The guide includes module/type design, principal `t` and receiver-first APIs, abstract representations and validated decoding, typed comparison, and `if`/pattern-match rules with RWO sources. Eio supersedes the older optional-runtime wording; pure data libraries can remain Eio-free. Historical experiments and the immutable evidence archive predate this standards update. Import these conventions into the new repository under <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue>; <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>/20/22 implement dependencies/tooling/CI and <issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue>/9 apply API/runtime contracts.

Snapshot: 2026-09-10. This document lets a new agent resume without the original conversation. Read live Linear status and comments before acting; the snapshot is not a substitute for subsequent user decisions or repository changes.

## Goal and current state

Build intuitive OCaml bindings to GPUI through a small Rust FFI, with a Jane Street Bonsai native application library. The v1 acceptance case is a polished agent-chat application: long/paged conversations, streaming responses, reliable native editing, Markdown/code/diff, attachments, search, tabs and multiple windows.

The feasibility experiments are complete enough to begin implementation. A production GPUIO repository has not been created as of this snapshot. The existing `/Users/dakotamurphy/chatgpt` repository is a different application and has another agent working in it. Only its ignored research notes were used. No existing opam switch was modified by the v0.17 experiment.

Live audit at this snapshot: <issue id="45d6f53a-e819-4545-abf0-16087d587500" href="https://linear.app/ochat/issue/OCH-5/gpui-research">OCH-5</issue> is In Progress; <issue id="a09e4a7c-4984-4698-aa4e-96516c4fddaa" href="https://linear.app/ochat/issue/OCH-6/establish-reproducible-gpuio-builds-on-macos-and-linux">OCH-6</issue> through <issue id="28db0e33-5616-424d-872d-75a9e72c3035" href="https://linear.app/ochat/issue/OCH-22/set-up-macos-and-linux-ci-repository-checks-and-clean-checkout">OCH-22</issue> are Backlog. <issue id="a09e4a7c-4984-4698-aa4e-96516c4fddaa" href="https://linear.app/ochat/issue/OCH-6/establish-reproducible-gpuio-builds-on-macos-and-linux">OCH-6</issue> is a tracking parent; <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue> through <issue id="28db0e33-5616-424d-872d-75a9e72c3035" href="https://linear.app/ochat/issue/OCH-22/set-up-macos-and-linux-ci-repository-checks-and-clean-checkout">OCH-22</issue> are its concrete setup children. No tickets have implementation PRs or completed production features in this research handoff.

## Read in this order

Before the numbered design/evidence sequence, read the [OCaml engineering standards](<https://linear.app/ochat/document/gpuio-ocaml-engineering-standards-core-eio-and-jane-street-conventions-38f45d0fbed5>), including Core-first precedence, Eio, naming/type/module/function-design conventions, testing and reliability guidance.

1. [Accepted scope and API contracts](<https://linear.app/ochat/document/gpuio-accepted-scope-and-api-contracts-e362ea30397f>).
2. [Architecture and v1 design](<https://linear.app/ochat/document/gpuio-architecture-and-v1-implementation-design-a3dd5c93b72f>).
3. [Validated v0.17 experiment](<https://linear.app/ochat/document/gpuio-validated-bonsai-v017-native-experiment-9e0212bfcff4>).
4. [Implementation/setup backlog](<https://linear.app/ochat/document/gpuio-milestones-and-implementation-backlog-b05bf6de21ab>), then the assigned or first unblocked issue.
5. Follow task-specific references to the [API/parity sketches](<https://linear.app/ochat/document/gpuio-ocaml-api-examples-and-gpuix-parity-44a1bde567b3>), [dependency/source manifest](<https://linear.app/ochat/document/gpuio-dependency-baseline-version-comparison-and-source-revisions-963d41122da4>) and [codec experiment](<https://linear.app/ochat/document/gpuio-bin-prot-interoperability-and-wire-format-experiment-3d5010be13a5>).

The [research index](<https://linear.app/ochat/document/gpuio-research-and-design-index-c32e88385f39>) links all documents and evidence. Historical preview research is a diagnostic reference, not the selected dependency baseline. API examples are illustrative and do not describe an already implemented SDK. Latest user decisions and live issue updates resolve conflicts with older research snapshots.

## Decisions already made

* Stock OCaml 5.3, Bonsai v0.17 and the v0.17 Jane Street dependency family; defer OxCaml.
* The experimental native library subset builds with Dune 3.24.2. Small identifier/native-packaging fork changes are acceptable; no Bonsai lifecycle/Incremental runtime patch is needed in the selected baseline.
* macOS and Linux together for v1; no Windows target. Validate both Wayland and X11, not just Linux compilation.
* GPUIX parity means components and application-building functionality. Hot reload, React Refresh and native-code reload are not requirements.
* Typed OCaml layout/content/control APIs and styles with useful accessible defaults; native hover/focus/pressed behavior and animations.
* Rust owns the live editor, IME, selection, undo, focus and immediate scrolling/animation. OCaml owns application decisions/data. Observations do not implicitly overwrite native edits; commands and submitted snapshots have explicit semantics.
* Persistent application data is distinct from disposable row presentation. Retain focused/composing/selected resources explicitly; row visibility must not cancel conversation work.
* Small versioned bin_prot command/event boundary; no synchronous OCaml calls from native paint/layout/input. One OCaml UI domain initially with explicit per-window driver/task scopes.
* Eio is required for first-party OCaml I/O and task integration; pure data libraries may remain Eio-free. No Async runtime is required.

Do not reopen these just because newer dependency releases exist. If evidence requires changing an accepted contract, record the concrete incompatibility/tradeoff and update the decision record deliberately.

## First implementation work

If resuming implementation from this snapshot, start with [OCH-18: repository scaffold](<https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold>). Query live project resources first so a repository created since this handoff is reused rather than duplicated.

Order: <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue> → <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue> dependency/fork packaging → <issue id="0ab5c2ea-d82d-4cf3-8d9b-b6a4edb04851" href="https://linear.app/ochat/issue/OCH-20/provide-an-isolated-development-environment-and-contributor-bootstrap">OCH-20</issue> environment and <issue id="b7861ca7-32fc-4406-bfa5-6ea4a0954e43" href="https://linear.app/ochat/issue/OCH-21/integrate-the-dune-and-cargo-build-and-add-cross-platform-native-smoke">OCH-21</issue> native build integration → <issue id="28db0e33-5616-424d-872d-75a9e72c3035" href="https://linear.app/ochat/issue/OCH-22/set-up-macos-and-linux-ci-repository-checks-and-clean-checkout">OCH-22</issue> CI. <issue id="a09e4a7c-4984-4698-aa4e-96516c4fddaa" href="https://linear.app/ochat/issue/OCH-6/establish-reproducible-gpuio-builds-on-macos-and-linux">OCH-6</issue> completes when its children and cross-platform foundation acceptance checks pass. <issue id="a178ea7f-53c7-46e3-8ff3-fccf48d574f1" href="https://linear.app/ochat/issue/OCH-7/implement-the-versioned-rust-bridge-and-atomic-retained-tree">OCH-7</issue> production protocol follows that gate; <issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue> API and <issue id="a27fb666-90b9-4196-9bfc-8bb11b1c1c5f" href="https://linear.app/ochat/issue/OCH-9/implement-bonsai-lifecycle-scheduling-and-optional-eio-task-scopes">OCH-9</issue> scheduling follow it. The early native smoke example can reuse the research bridge without implementing the final public API/protocol, avoiding a dependency cycle.

Use one repository and release train initially. Migrate the accepted decisions/specs into versioned docs; keep the Linear links connected to reviewed commits. Grow the reference app and native tests through each milestone, rather than waiting until <issue id="5ce1cdb3-1325-44e0-bed0-5e5a5bef9a28" href="https://linear.app/ochat/issue/OCH-16/build-the-agent-chat-reference-application-as-the-v1-acceptance">OCH-16</issue> to write the first application code.

## Source and evidence map

Same-machine research root: `/Users/dakotamurphy/gpuio-research-2026-09-10`. Notes: `/Users/dakotamurphy/chatgpt/scratch/gpuio-research`.

| File/directory under research root | Purpose | Migration guidance |
| -- | -- | -- |
| `native-v017/main.ml` | Current Bonsai/Eio host, reconciliation, lifecycle scheduling, self-test | Use this OCaml baseline, not the older preview [main.ml](<http://main.ml>) |
| `native-v017/native.ml`, `wire.ml`, `dune` | FFI declarations, experimental wire types, linking configuration | Port into the proper library/build layout; schemas are not frozen |
| `native-v017/lifecycle_check.ml` | Standalone keyed-state/lifecycle regressions | Preserve as a focused regression when packaging dependencies |
| `native-v017/evidence/manifest.json` and `*-native-v017.patch` | Eight v0.17 upstream commits, patch hashes and native build selection | Reconstruct vendored sources and verify patches rather than guessing versions |
| `native-v017/evidence/native-self-test-final.log` | Successful current native run | Includes unchanged-tree lifecycle regression and clean cancellation/shutdown |
| `native-spike/rust/src/{bridge,protocol,tree,text_input,lib}.rs` | Native bridge, decoder, retained tree, text handler, GPUI host | This Rust code is also the implementation used by the v0.17 run |
| `native-spike/rust/Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | Exact Rust dependency/toolchain inputs | Replace relative paths to external research checkouts with reproducible project dependencies |
| `native-spike/THIRD_PARTY.md`, `LICENSE-APACHE` | Source attribution/license provenance | Preserve attribution when moving code |
| `codec-spike/` | Independently constructed OCaml/Rust binary fixtures and tests | Codec evidence; not a complete production message schema |
| `sources/` | Pinned upstream repositories inspected during research | Not included in the uploaded archive; restore from manifests |

The `.rs` list above denotes separate files, not a literal filename. The v0.17 Rust archive was built against the same stock OCaml 5.3 runtime and copied from the earlier native spike. It is omitted from the evidence download and must be rebuilt from source on a new machine.

## Recovering without this machine

1. Retrieve the research/design ZIP attached to [OCH-5](<https://linear.app/ochat/issue/OCH-5/gpui-research>). If a signed attachment URL expires, retrieve its current attachment URL again from Linear.
2. Verify archive SHA-256 `0b9e739b46db04bdeb5be232745116cc7bbb82ce5b5ddfc8dac7b9aba3f04cff`. It contains 147 entries; `FILE-MANIFEST.json` gives per-file checksums and `BUNDLE-README.md` explains exclusions/chronology.
3. Extract to a new dedicated workspace. `notes/` contains the notes that originally lived in the other application repository. `native-v017/`, `native-spike/` and `codec-spike/` preserve experiment sources/evidence.
4. Restore upstream repository commits from `notes/source-manifest.json` and the eight v0.17 entries in `native-v017/evidence/manifest.json`. The latter packages use their corresponding `janestreet/<package>` repositories. Apply the matching native-v017 patches and check their hashes. Full GPUI workspace dependencies may require more than just copying `crates/gpui`.
5. Create an isolated, pinned compiler/dependency environment and rebuild the Rust archive. Adapt the historical absolute paths and external Cargo dependency paths. <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>/<issue id="0ab5c2ea-d82d-4cf3-8d9b-b6a4edb04851" href="https://linear.app/ochat/issue/OCH-20/provide-an-isolated-development-environment-and-contributor-bootstrap">OCH-20</issue>/<issue id="b7861ca7-32fc-4406-bfa5-6ea4a0954e43" href="https://linear.app/ochat/issue/OCH-21/integrate-the-dune-and-cargo-build-and-add-cross-platform-native-smoke">OCH-21</issue> implement this reproducibility work; the archive is not already a portable installer.
6. Preserve the old evidence as historical; record new build/test results with the actual environment and source revisions.

Rust inputs used: Rust 1.97.1; upstream GPUI commit `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`; ocaml-interop commit `9f941c63b8a9dcd652c6535da196f48d7a4dcc75` with `no-caml-startup`; binprot-rs commit `165b4d64d8f2a580af453f1e60deba41c28cd6df`. GPUIX reference commit is `18e695ed0ee8121a7793413ca795e08eda2a13df`. GPUIO uses upstream GPUI; it has not adopted GPUIX's Zed fork.

Do not use `native-spike/prepare-environment.sh` as the v0.17 bootstrap: it prepares the historical preview stack. Do not run a global/default-switch upgrade to make old commands work.

## Same-machine baseline checks

After checking the existing compiler/dependencies read-only, the current experiment can be checked with:

```sh
cd /Users/dakotamurphy/gpuio-research-2026-09-10/native-v017
./build.sh
./_build/default/lifecycle_check.exe
./run.sh --self-test
```

`build.sh` reads the existing `default` switch and invokes Dune 3.24.2 from the separate research root. It does not install packages. If that environment has since changed, reproduce the pinned dependencies in a separate environment rather than altering another project's switch. Keep builds outside the unrelated application repo, including when Dune's recursive discovery would otherwise find them.

Expected evidence markers include `LIFECYCLE_PASS`, `unchanged_tree_lifecycles_passed=true`, `SELF_TEST_PASS`, balanced 23 row activations/deactivations after shutdown, `inflight_eio_cancelled=true` and `shutdown_complete`. The deliberately induced Rust panic is caught by the boundary test; its printed diagnostic is expected when followed by the containment/pass markers. Do not compare exact timing numbers as pass criteria.

## What is established, and what is not

| Established in the research run | Not established by that evidence |
| -- | -- |
| Native GPUI/OCaml/Bonsai integration on macOS 14.5 arm64 | Linux runtime support or final minimum OS/architecture policy |
| Codec fixture equality; bounded experimental decoder/atomic-tree tests | Complete production protocol evolution, queue overload or resource security guarantees |
| Keyed state, lifecycle ordering, unchanged-tree transitions and cleanup | Bounded retained Bonsai models after traversing a huge managed list |
| Native input-handler composition/UTF-16 callbacks and revision checks | Real OS IME automation, full editor/undo/accessibility parity |
| Eio file work/cancellation and no Async symbols in the executable audit | Final scheduler design or exhaustive dependency/runtime proof |
| A single live native window with interactive rows/text | Multi-window application, managed list, rich-document or animation implementation |

The historical screenshot is from the preview run. The v0.17 logs are the selected baseline. The earlier preview graph-initialization failure was not conclusively classified as an upstream bug or compiler problem. It is not necessary to solve it before proceeding with v0.17.

The original `installed-packages.json` contains opam invocation metadata, not package versions. The added publication-time inventory is labelled separately. Do not claim it is an exact replay of the earlier environment.

## Prototype shortcuts that must not become accidental API contracts

| Observed shortcut | Replacement / issue |
| -- | -- |
| Absolute build paths and copied Rust archive | Source-built, repository-relative tooling: <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>–<issue id="b7861ca7-32fc-4406-bfa5-6ea4a0954e43" href="https://linear.app/ochat/issue/OCH-21/integrate-the-dune-and-cargo-build-and-add-cross-platform-native-smoke">OCH-21</issue> |
| One process-global bridge/one window and simple numeric IDs | Per-window identity, generations and explicit ownership: <issue id="a178ea7f-53c7-46e3-8ff3-fccf48d574f1" href="https://linear.app/ochat/issue/OCH-7/implement-the-versioned-rust-bridge-and-atomic-retained-tree">OCH-7</issue>/<issue id="52a46bbe-65d7-4ffe-8808-86f79e48a88d" href="https://linear.app/ochat/issue/OCH-15/implement-multi-window-workspaces-tabs-and-scoped-application-commands">OCH-15</issue> |
| Command channel bounded at 64, but native event VecDeque unbounded | Bounded event semantics and overload/backpressure behavior: <issue id="a178ea7f-53c7-46e3-8ff3-fccf48d574f1" href="https://linear.app/ochat/issue/OCH-7/implement-the-versioned-rust-bridge-and-atomic-retained-tree">OCH-7</issue> |
| Full retained-tree clone when staging a batch; full OCaml tree flatten/diff | Validate/apply touched data and measured reconciliation improvements: <issue id="a178ea7f-53c7-46e3-8ff3-fccf48d574f1" href="https://linear.app/ochat/issue/OCH-7/implement-the-versioned-rust-bridge-and-atomic-retained-tree">OCH-7</issue>/<issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue> |
| Handler IDs tied to node IDs; old click revision not used for generation validation | Independent generation-checked event registrations and stale-event tests: <issue id="a178ea7f-53c7-46e3-8ff3-fccf48d574f1" href="https://linear.app/ochat/issue/OCH-7/implement-the-versioned-rust-bridge-and-atomic-retained-tree">OCH-7</issue>/<issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue> |
| Permanent cancellable 60 Hz Eio tick | Input/completion/deadline/frame-driven wakeups: <issue id="a27fb666-90b9-4196-9bfc-8bb11b1c1c5f" href="https://linear.app/ochat/issue/OCH-9/implement-bonsai-lifecycle-scheduling-and-optional-eio-task-scopes">OCH-9</issue> |
| List of a few ordinary rows, not a managed virtual list | Native measurement/paging plus explicit model/cache eviction: <issue id="156ccdcd-d8bf-4fee-aa50-f0b0e0f2467f" href="https://linear.app/ochat/issue/OCH-13/implement-managed-virtual-lists-with-paging-and-explicit-state">OCH-13</issue> |
| Minimal text-handler example, not a complete editor | Editor command/observation contract and real OS tests: <issue id="75788943-a521-438e-92c7-f3567d08835a" href="https://linear.app/ochat/issue/OCH-10/implement-native-text-editing-with-explicit-observation-and-command">OCH-10</issue> |
| macOS-specific loop-stop workaround for clean return/join | Explicit platform close/quit design and multi-window tests: <issue id="52a46bbe-65d7-4ffe-8808-86f79e48a88d" href="https://linear.app/ochat/issue/OCH-15/implement-multi-window-workspaces-tabs-and-scoped-application-commands">OCH-15</issue>/<issue id="b7861ca7-32fc-4406-bfa5-6ea4a0954e43" href="https://linear.app/ochat/issue/OCH-21/integrate-the-dune-and-cargo-build-and-add-cross-platform-native-smoke">OCH-21</issue> |
| Diagnostic latency numbers and finite GC cycles | Named hardware, representative workloads and resource budgets: <issue id="8afec95a-ba31-426f-b8fb-78dfea2c5447" href="https://linear.app/ochat/issue/OCH-17/complete-component-parity-platform-validation-and-v1-distribution">OCH-17</issue> |

Neither GPUI nor a Rust component dependency automatically provides the whole Zed editor. Evaluate reusable widget behavior against our pinned GPUI before adopting it. Longbridge's gpui-kit/gpui-base/gpui-component is a candidate, not an already chosen dependency. Dependency reuse must preserve the accepted ownership/API contracts.

## Open decisions and their owners

* Repository owner/name/visibility and project license: <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue>. Check live user/account/project context; these details were not selected in research.
* OCaml dependency lock/package distribution strategy and exact native fork packaging: <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>.
* Supported OS minimums and architecture artifacts, graphical runner availability: <issue id="a09e4a7c-4984-4698-aa4e-96516c4fddaa" href="https://linear.app/ochat/issue/OCH-6/establish-reproducible-gpuio-builds-on-macos-and-linux">OCH-6</issue>/<issue id="b7861ca7-32fc-4406-bfa5-6ea4a0954e43" href="https://linear.app/ochat/issue/OCH-21/integrate-the-dune-and-cargo-build-and-add-cross-platform-native-smoke">OCH-21</issue>/<issue id="28db0e33-5616-424d-872d-75a9e72c3035" href="https://linear.app/ochat/issue/OCH-22/set-up-macos-and-linux-ci-repository-checks-and-clean-checkout">OCH-22</issue>.
* Exact public names/signatures and style precedence/inheritance/unset behavior: <issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue>.
* Editor/control reuse and revision command behavior: <issue id="75788943-a521-438e-92c7-f3567d08835a" href="https://linear.app/ochat/issue/OCH-10/implement-native-text-editing-with-explicit-observation-and-command">OCH-10</issue>/<issue id="448e874f-dc9e-420d-9df7-0ff08dcdef49" href="https://linear.app/ochat/issue/OCH-11/implement-accessible-controls-actions-and-native-interaction">OCH-11</issue>.
* Model eviction, focus/selection retention and paging API mechanics: <issue id="156ccdcd-d8bf-4fee-aa50-f0b0e0f2467f" href="https://linear.app/ochat/issue/OCH-13/implement-managed-virtual-lists-with-paging-and-explicit-state">OCH-13</issue>.
* Huge-document selection/parser invalidation/cache policy: <issue id="8da7ccd9-64cc-4cbc-abe2-d89e6df17536" href="https://linear.app/ochat/issue/OCH-14/implement-revisioned-streaming-documents-and-markdown-code-and-diff">OCH-14</issue>.
* Quantitative performance/cache budgets and full platform validation: <issue id="8afec95a-ba31-426f-b8fb-78dfea2c5447" href="https://linear.app/ochat/issue/OCH-17/complete-component-parity-platform-validation-and-v1-distribution">OCH-17</issue>.

These are bounded implementation decisions, not hidden prerequisites for rediscovering feasibility. Do not silently treat illustrative snippets, proposed targets or optional features as already implemented requirements.

## What to leave for the next agent

For each completed or paused issue, record the repository/branch/commit or PR, changed public contracts, exact build/test commands and environment, results/limitations, new source/patch revisions, and the next concrete step. Distinguish compile-only, synthetic native-handler and real OS validation. Update the relevant spec/decision record when an accepted contract changes; avoid making later agents infer the current design from a long chain of contradictory comments.

Do not check in secrets, local switches or build artifacts. Keep concurrently used environments isolated. Repository creation and implementation are the next tasks, not work performed by this handoff publication.

## Copyable resume prompt

> Resume GPUIO using the Linear research/design index and the “Start here: agent handoff” document. Read the accepted contracts, OCaml engineering standards, current v0.17 evidence and live issue dependencies/statuses. Continue the assigned task, or start <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue> if setup has not begun. Use stock OCaml/Bonsai v0.17 and target macOS plus Linux. Preserve the existing application's switch and workspace. Treat historical preview workarounds and illustrative API sketches appropriately. Implement a reviewable vertical slice, validate it on the relevant platforms, and leave an issue update with commits, evidence, unresolved choices and the next step.

## Original handoff audit (before expanded-v1 tickets)

On 2026-09-10, all 17 implementation/setup issues (<issue id="a09e4a7c-4984-4698-aa4e-96516c4fddaa" href="https://linear.app/ochat/issue/OCH-6/establish-reproducible-gpuio-builds-on-macos-and-linux">OCH-6</issue> through <issue id="28db0e33-5616-424d-872d-75a9e72c3035" href="https://linear.app/ochat/issue/OCH-22/set-up-macos-and-linux-ci-repository-checks-and-clean-checkout">OCH-22</issue>) were checked for acceptance criteria and design/research references. All had both. The prerequisite graph, including the foundation parent's dependence on its setup children, is acyclic. Thirteen relevant issues now also include explicit source/migration starting points and prototype caveats. <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue> is the first unblocked setup issue at this snapshot. This audit verifies planning completeness, not implementation correctness.

## Expanded-v1 handoff audit

Rechecked on 2026-09-10 after the scope expansion: 27 implementation/setup issues, comprising 24 required items and 3 optional platform packages. All have acceptance criteria or an explicit parent completion gate and research/design references. The prerequisite graph, including parent completion gates, is acyclic. <issue id="8afec95a-ba31-426f-b8fb-78dfea2c5447" href="https://linear.app/ochat/issue/OCH-17/complete-expanded-v1-capabilities-platform-validation-and-distribution">OCH-17</issue>'s prerequisite closure includes every required expansion ticket (<issue id="3784f93e-f408-45d2-832d-8b0c822b8255" href="https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk">OCH-23</issue>–<issue id="65ca62fd-3ae5-4737-bdb7-d38697f6e6fc" href="https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently">OCH-29</issue>) and excludes optional <issue id="0f9f4312-8b52-4a88-a6d1-7457fe22677a" href="https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration">OCH-30</issue>–<issue id="a9c40d72-c648-4536-a74a-b4aa897c7eef" href="https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces">OCH-32</issue>. <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue> remains the first setup task; <issue id="5ce1cdb3-1325-44e0-bed0-5e5a5bef9a28" href="https://linear.app/ochat/issue/OCH-16/build-the-agent-chat-reference-application-as-the-v1-acceptance">OCH-16</issue> remains the working chat milestone before expansion. There are seven required milestones plus a separate optional-platform milestone. No deadlines were introduced. This is a planning audit, not proof of implemented capabilities.

