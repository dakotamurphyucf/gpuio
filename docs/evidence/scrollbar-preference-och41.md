# Scrollbar preference snapshot — OCH-41

The [2026-10-08 native follow-up](#actual-appkit-and-gallery-follow-up--2026-10-08)
below adds both actual AppKit style results and public gallery application.
The original checkpoint retains its narrower evidence and dated open items.

Checkpoint 2026-10-04, macOS arm64. Working tree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`; substantial uncommitted changes mean
that HEAD alone does not identify these sources. OCH-41/OCH-17 remain open.

## Source and contract

The [contract](../design/scrollbar-preference.md) adds a single asynchronous
desktop query and an explicit Collections gallery action. No dependencies,
vendor patches or global toolchain settings changed in this slice.

Pinned Zed `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` implements
`App::should_auto_hide_scrollbars` by forwarding to the platform. The macOS
implementation in `crates/gpui_macos/src/platform.rs` reads
`NSScroller.preferredScrollerStyle`. Linux's `crates/gpui_linux/src/linux/platform.rs`
initializes `auto_hide_scrollbars` to false and only exposes its getter; no
updater was found in the pinned Linux source. We return `Unsupported` on Linux
instead of treating this default as a measurement. Component's snapshotted
[`sync_scrollbar_appearance`](../catalog/sources/component-theme.rs.txt)
is likewise an explicit read/sync operation, not a subscription.

The query reports resolved overlay/legacy behavior, not macOS's raw three-way
System Settings choice. The gallery maps it to Scrolling/Always respectively,
preserving manual choice on error. It uses a fresh child scope on activation,
checks that scope before applying a response, and guards one pending request.
Explicit mode actions also check scope/busy state when executed. Departed-page
responses cannot update a new activation. No automatic refresh or polling is
claimed; the example asks the user to apply the preference again after changes.

## Validation

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless stated otherwise.
Scratch logs are under `scratch/agents/root-20261003-release-notices/` and are
not build dependencies.

- `dune build -j2 @runtest examples/gallery/main.exe`: passed, including the
  full OCaml suite and independent gallery backend. Log
  `scrollbar-preference-ocaml-001.log`. OCaml checks independent request/event
  bytes, both returned styles and invalid tags, truncation, extra bytes and
  zero correlation.
- `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2`:
  **906 passed, two existing macOS private-bus skips**. Log
  `scrollbar-preference-native-001.log`. The new policy test reads each snapshot
  afresh and never invokes an unsupported getter. The production dispatch test
  services two reserved requests through TestPlatform, preserves correlation,
  uses no desktop identity or window, and leaves no output after draining.
- `cargo test -p gpuio-protocol --offline --locked -j2`: **390 passed**, including
  eight desktop tests. Log `scrollbar-preference-protocol-001.log`. Rust fixes
  the same independent bytes, preserves all preceding request fixtures and
  checks malformed/truncated/trailing request input.
- `./scripts/gpuio check-fmt`: passed, log
  `scrollbar-preference-format-check-001.log`. Only formatter output was
  promoted; no expectation output was promoted. Catalog audit, relative links
  in seven edited documents and `git diff --check` passed. Structural catalog
  coverage remains 146 module entries in 43 families.
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`:
  passed, log `scrollbar-preference-clippy-001.log`.

The existing `block 0.1.6` future-compatibility notice and macOS duplicate-library
link warnings remain. The tests ran locally on macOS; no fresh Linux execution,
OS preference read, physical window or hosted check is included.

## Remaining acceptance

TestPlatform's getter is a fixed false. Dispatch tests therefore establish
routing and ownership, not a read of the developer's actual macOS setting.
Physical System Settings changes, both style results, applying while scrolled,
keyboard/AX presentation, page departure and multi-window walkthrough remain
unverified. Local source guards are not physical lifecycle evidence.
Linux nongraphical checks still need execution on Linux; Linux desktop
qualification remains OCH-47. The full gallery and release gates remain open.

## Actual AppKit and gallery follow-up — 2026-10-08

`scripts/test_macos_scrollbar_preference.py` now runs three separate gallery
processes on macOS 14.5 arm64: the current setting and process-local argument
defaults `-AppleShowScrollBars Always` / `WhenScrolling`. An independently
compiled AppKit helper reads `NSScroller.preferredScrollerStyle` with the same
arguments and verifies legacy/overlay respectively. No global/defaults database,
input source, clipboard or other OS setting is written. These are actual AppKit
queries and native windows, not TestPlatform substitutions.

All three cases pass. Current and WhenScrolling resolve to `Auto_hide`, mapped
to While scrolling; Always resolves to `Always_visible`, mapped to Always visible.
The driver starts with the opposite manual choice before applying the native
snapshot. Captures show the expected selected mode and status. A real native Down
key moves the vertical range from 0 to 26 logical pixels; applying the snapshot
keeps 26. End reaches 677 and Home restores zero after the mode update. Leaving
and returning to the page permits a new manual choice and fresh native read.
All three owned applications close normally and exit zero.

The first attempt failed on a fixture assumption after the snapshot/keys passed:
it expected page reactivation to clear the already published status. Bonsai
retains that model while `Preview_scope` cancels/recreates the request scope.
The corrected fixture preserves this distinction and tests a fresh successful
query after an explicit choice. No production change was required. This is not
evidence for deliberately delayed in-flight response cancellation, multi-window
isolation, live System Settings notifications, pointer dragging, fade timing,
VoiceOver or full scrollbar/table acceptance.

The [native reports archive](scrollbar-preference-och41/native-reports.tar.gz)
and [manifest](scrollbar-preference-och41/native-manifest.json) preserve both
attempts, exact fixture/oracle sources, reports, build logs and window captures;
every file was verified by reading the archive back. Source base is
`3b929690936d61b59027882bc02894c326644195`, gallery SHA-256
`2694c047b64f750f17758ca521c3b5c7bb4fa70f13f9135f3a334adecdd382ee`.

```sh
python3 scripts/test_macos_scrollbar_preference.py --output <fresh-directory>
python3 -m py_compile scripts/test_macos_scrollbar_preference.py
```

The corrected run and syntax checks pass, as do workflow actionlint, catalog and
example documentation audits, and diff checks. Foundation now includes this
bounded native scenario; hosted acceptance of the new revision remains pending.

## Hosted oracle initialization repair — 2026-10-08

Foundation [37758529147](https://github.com/dakotamurphyucf/gpuio/actions/runs/37758529147)
fails the first/current preference case: the helper expects `Auto_hide`, but the
public gallery completes its query and displays `Always_visible`. This is an
actual disagreement, not a pending query or a caption that merely needs more time.
The production getter directly invokes AppKit's `NSScroller.preferredScrollerStyle`.

A standalone Swift diagnostic isolates AppKit initialization, without compiling
GPUIO or writing OS settings. On the local macOS 14.5 arm64 machine, current and
Automatic remain overlay throughout initialization. On hosted macOS 15 arm64,
[run 37773298062](https://github.com/dakotamurphyucf/gpuio/actions/runs/37773298062)
records current/Automatic as overlay before and immediately after `finishLaunching`,
then legacy after a 250 ms event-loop interval. Always and WhenScrolling remain
legacy and overlay respectively. The cold helper exits before this automatic
policy resolution; the later gallery snapshot agrees with initialized AppKit.

Apple documents that the resolved style depends on the user setting and connected
pointing devices and can change over time. This evidence establishes the startup
transition on this runner, not a universal 250 ms initialization guarantee or the
identity of a particular device. [AppKit reference](https://developer.apple.com/documentation/appkit/nsscroller/preferredscrollerstyle).

The helper now calls `finishLaunching`, runs the event loop for a fixed 0.5 seconds,
and returns JSON containing startup and initialized styles, the effective defaults
value and the interval. The Python driver compares the gallery against the
initialized result and preserves both values in its report. Explicit legacy and
overlay expectations remain strict. This is a fixed initialization interval,
not a retry until the expected answer; the production snapshot API is unchanged.
Live preference/device changes during a run can still invalidate an independently
sampled expectation and should not be treated as successful qualification.

[Hosted follow-up 37773657349](https://github.com/dakotamurphyucf/gpuio/actions/runs/37773657349)
passes the corrected oracle against the independent phase probe in all four cases.
The local full gallery walkthrough passes all three current/legacy/overlay cases,
including exact expected status, native Down/End/Home, retained scroll position
and a fresh read after page reactivation. All three applications exit zero.
No System Settings, clipboard or input-source state changes. The hosted follow-up
runs only AppKit probes; the full corrected gallery step still needs its new
Foundation run. The live Foundation at `6be17496` predates this helper correction.

Commands:

```sh
python3 scripts/test_macos_scrollbar_preference.py --output <fresh-directory>
python3 -m py_compile scripts/test_macos_scrollbar_preference.py
ruff check scripts/test_macos_scrollbar_preference.py
```

These checks, example/catalog audits, actionlint, local documentation links and
whitespace checks pass. No new OCaml/Rust build or unit suite is needed for this
Swift/Python fixture correction. Local gallery executable remains the
[focus-forwarding checkpoint](window-focus-forwarding-och17.md). The remote probe
uses an isolated diagnostic branch and workflow; that replacement workflow must
not be merged into the release branch.

The [diagnostic archive](scrollbar-preference-och41/initialization-reports.tar.gz),
[verified manifest](scrollbar-preference-och41/initialization-manifest.json) and
[summary](scrollbar-preference-och41/initialization-summary.json) preserve both
hosted probe outputs, the local gallery result, exact helper/driver/diagnostic
sources and workflow. Full accessibility, live preference notifications, physical
fade behavior and all remaining release gates remain separate.
