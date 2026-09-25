# Milestone 04 — Agent workspace

Local implementation and acceptance ledger for OCH-14, OCH-15 and OCH-16.
Platform: macOS arm64; stock OCaml 5.3, Bonsai/Core v0.17, Eio 1.3, Dune 3.24.2,
Rust 1.97.1. All commands select this repository's isolated toolchain. No other
switch or project was modified. Consolidated validation and hosted results are recorded below.
[PR #12](https://github.com/dakotamurphyucf/gpuio/pull/12) records the final checked
head and merge; Linear records delivery and the remaining OCH-17 platform work.

## Scope and evidence

| Ticket | Implemented behavior | Principal checks |
| --- | --- | --- |
| OCH-14 | Revisioned UTF-8 document resources; bounded/coalesced append/edit/reset; native Markdown, highlighted code and unified diff; explicit images/navigation; selection/copy/search; source paging and bounded workers | Core/registry expect tests, independent Rust/OCaml fixtures, resource/worker/diff/search/highlight tests, `native_document`, public documents app and integrated streams |
| OCH-15 | Independent window drivers and shared stores; configured windows and observations; correlated commands; async close/quit decisions; reopen; retained tabs and native split panes | Workspace/reconciliation/lifecycle expect tests, independent fixtures, `native_window`, `native_tabs`, `native_split`, public window-lifecycle app and two-window reference app |
| OCH-16 | Sidebar search, paged transcript, native composer, rich/tool artifacts, text attachments, retained tabs/windows, commands/themes, deterministic concurrent streaming/cancel/retry | Pure fake-backend expect tests, public agent-workspace self-test, external macOS AX/keyboard/picker driver, rendered dark/light/code/palette/dialog inspection |

The [document contract](../design/documents.md), [window contract](../design/windows.md)
and [reference-app ownership design](../design/agent-workspace.md) specify limits
and semantics. The [example README](../../examples/agent_chat/README.md) gives
runnable commands and the application/developer interface.

## Local checks

Component-family validation passed before integration: full OCaml/Rust suites,
independent wire fixtures, Clippy with warnings denied, formatting, native document,
window, tab and split scenarios, and public document/window examples. The combined
native controls suite passed after retained-panel visibility/focus changes.

The reference-app controller scenario opens real GPUI windows and checks:

- Exact native submission and protection of a draft edited during send acceptance.
- Two concurrent responses, cancellation with partial text, and generation-reset retry.
- Streaming while the relevant row is offscreen; history prepending retains row175's anchor.
- Actual hide/reopen of a retained panel, preserving draft and scroll state.
- Shared conversation contents with independent native drafts/viewports in two windows.
- Closing one window cancels its pending acceptance without producing a late response;
  the survivor remains usable and conversation work retains its own scope.
- Valid attachment registration and binary/oversize rejection; simulated failure/retry.
- Theme changes, requested native frames and orderly application shutdown.

The external macOS test (`scripts/test_agent_chat.py`) targets only its child PID.
It uses native accessibility activation/value plus targeted Return,
Command-Shift-P and Escape. It checks sidebar search; Send; delayed-send draft
protection; native Enter/error/retry; retained tabs; independent windows; actual
AppKit picker selection followed by Eio read/preview; theme and keyboard palette;
and OS close deny/allow. File selection uses a real double-click at the exact
file element’s accessible bounds, after verifying the screen point belongs to
the test child; an occluding application causes failure. Success requires the process to return through `App.run`
and emit `GPUIO_AGENT_CHAT_NATIVE_APP_RETURNED`. Failure terminates/reaps its child.
This test requires macOS Accessibility authorization; a missing permission is not
an application success. No unrelated desktop process or file is targeted.

The native document harness measures actual GPUI layout/paint using a background
window, including Unicode code selection under append, Markdown/table/fence and
explicit image handling, select-all snapshot behavior, diff navigation/folding,
full-source search/huge-source fallback, and lease disposal. These checks do not
claim physical display presentation or complete OS IME/screen-reader acceptance.
The separate native editor suite covers marked composition and Read_snapshot,
including retained hidden editors. Tab/split tests cover native keyboard and macOS
accessibility actions; split tests also exercise pointer ownership/cancellation.

## Measurements

The worker report separates queue, parsing, highlighting and search time from
native layout/paint and process memory. The final local document run processed 100,570 source bytes across six jobs: 9,097µs queue,
863µs parse, 12,468µs highlight, 652µs search; peak measured worker reservations
3,210,432 units and at most one active worker for that sequential workload.
Native layout/paint totaled31,501µs; process peak RSS was62,849,024 bytes.
These are debug-build workload measurements, not per-frame latency or an FPS claim.
The worker pool independently tests two-worker/aggregate-budget limits and stale
superseded results. The registry's 1,000 two-byte appends to a100k baseline produce
one coalesced2,000-byte suffix with exact terminal content.

The integrated public app self-test (`/usr/bin/time -l
_build/default/examples/agent_chat/main.exe --self-test`) took3,942ms within the
scenario (4.81s process wall time), including deliberate fake-backend delays. It
reported55 commits,41 rendered acknowledgements,1,638 turns,236 clock ticks and37
completed jobs. Peak RSS was130,613,248 bytes (124.6MiB); macOS reported peak memory
footprint177,539,904 bytes (169.3MiB). These are distinct OS accounting measures,
not retained heap size or steady-state idle CPU. No swap was reported.

Hosted measurements at implementation revision `aea1edb`,
[run 36081642291](https://github.com/dakotamurphyucf/gpuio/actions/runs/36081642291):

| Scenario | Queue µs | Parse µs | Highlight µs | Search µs | Source bytes | Worker peak | Reserved peak |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| macOS arm64 headless workers | 140190 | 3988 | 58285 | 7632 | 1080882 | 2 | 9438592 |
| macOS native document | 90663 | 3687 | 49959 | 750 | 100570 | 1 | 3210432 |
| Linux x64 headless workers | 168701 | 1835 | 76563 | 8320 | 1080882 | 2 | 9438592 |
| Linux X11 native document | 8067 | 675 | 24052 | 935 | 100570 | 1 | 3210432 |

The hosted macOS document scenario measured 124,908µs layout/paint,57,458,688bytes
peak process RSS and366ms scenario duration.

The X11 document test completed six jobs without discarded results, with39,410µs
layout/paint,199,565,312bytes peak process RSS and247ms scenario duration. The X11
public agent scenario passed in4,507ms with49 commits,30 rendered acknowledgements,
1,524 turns,270 clock ticks and37 completed jobs. These hosted software-rendering
results are workload observations, not physical-presentation or hardware budgets.

Both Linux graphical sequences remained informational failures: X11 reached and
passed document/chat scenarios, then found the pinned GPUI Linux title getter
returns an empty string in the window-lifecycle test. The host now retains its
configured/updated title explicitly; follow-up CI validates that correction.
Wayland stopped earlier at the known combobox clipboard assertion (`native_controls`),
before reaching M4. Compilation and headless workers do not establish GUI acceptance.

## Visual acceptance

The owner explicitly requested a beautiful modern showcase. The app uses original
vector icons, restrained lilac accents, semantic light/dark colors, a compact
sidebar/tab strip, a bounded reading column, expandable tool artifacts, proper
monospace code, and a prominent composer. Native tooltips, focus rings, progress,
wrapped failure feedback, command palette and unsaved-draft dialog complete the
interaction states. Actual macOS windows were inspected in both themes, with code
expanded and overlays open. Screenshots contain only this application's window;
they are evidence of native rendering, not assets used to render the UI.

## Platform and delivery status

macOS is the current functional acceptance platform. Linux builds and unit tests
remain required. X11/Wayland graphical jobs are informational and full Linux GUI
acceptance remains OCH-17, per the owner's explicit policy. There is no claim of
Linux native keyboard/IME/AX parity from a passing build. Docking, drag-to-detach,
OS-native tab groups, provider SDKs, credentials and durable conversation storage
are outside this milestone's baseline.

Final local consolidated checks passed on2026-09-24 (2026-09-25 UTC):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --features native-tests --all-targets -j 2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_document --test native_editor --test native_tabs --test native_split --test native_window --test native_controls -j 2
_build/default/examples/documents/main.exe --self-test
_build/default/examples/window_lifecycle/main.exe
_build/default/examples/agent_chat/main.exe --self-test
python3 scripts/test_agent_chat.py
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
```

Full workspace lint and the legacy bridge/typed-view examples also pass.
Delivery: [PR #12](https://github.com/dakotamurphyucf/gpuio/pull/12).
The PR retains all required-check runs and the immutable final merge revision.

The first hosted macOS run passed the public chat/controller scenario and preceding
native component tests, but the external driver stopped at the native attachment
picker: the runner used column view, whereas the test selected a list-view row.
The next run passed the full X11 sequence, but found that macOS15 also exposes
different row/cell parents. The driver now searches the exact filename across
view layouts and double-clicks its verified child-owned screen position, avoiding
row assumptions. This is an automation correction; native picker/Eio attachment
acceptance remains real and required. Final runs are linked in PR #12.
