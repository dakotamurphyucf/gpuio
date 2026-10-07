# Painted document selection — OCH-17

Local macOS arm64 work after `cd8f5e81`, 2026-10-07. This extends the rendered
Document reading tree with its current native selection. Guarded OS mutation,
reveal, cached-tree replay and full accessibility/release acceptance remain open.

## Implementation

TextView publishes after its native children paint and the selection adapter
registers. It uses the installed prepared text and retained native selection, or
captures the current pointer range. The semantic frame supplies the actual
Document ID and checked native run endpoints; equal-text/foreign preparation
identity checks remain in force. Failed mapping does not invent a caret.

GPUI accepts selection claims only during paint with accessibility active.
Claims expire each frame. Just before sending the final tree to the platform,
one traversal validates all claimed Documents and TextRuns. A position must be
within its run's character count and belong to the same independent text scope.
Hidden/disabled ancestors and nested input/Document/Terminal scopes cannot lend
their positions to the outer Document. Invalid claims clear the property; they
do not alter tree structure, native controls, keyboard focus or action handlers.
The returned Boolean means queued, not successfully validated or authorized.

This final-tree pass is linear in tree size when claims exist. It does not scan
the full tree separately for each Document. Native pointer selection/Copy stays
with the existing owners; the OCaml serialization protocol is unchanged.

## Validation

Four new native tests cover same-frame forward/backward/caret ranges, replacement
clearing, invalid indices, missing runs, nested Document/input/Terminal scopes,
hidden/disabled ancestry, explicit clearing and rejection outside paint. Additional coverage exercises the application’s literal HTML plugins with MDX disabled/enabled and the actual window-level Copy path for a terminal code block.
Initial fixture failures were an opaque-position equality comparison and assuming
AX was active during initial window creation. The corrected tests use validated
content coordinates and respect the accessibility activation gate. Clippy then
found an unnecessary clone of Copy data, corrected before the final pipeline.

The first actual macOS probe exposed selected-text attributes but also duplicated
literal HTML tags. The app’s framework-owned declared Text renderer and its atomic
wrapper both published reading runs. The repair suppresses only the inner runs,
retaining its native Label/value/bounds, painting and selection geometry. Existing
profile semantics and pointer-selection tests caught an initial overly broad
suppression; those tests remain unchanged.

The window Copy adapter deliberately trims rendered boundary whitespace while the
logical selection retains it. A focused regression checks the actual
TextSelection window API, TextView state and prepared selection separately. The
OS probe compares exact text with the one terminal logical separator expected for
its fixed final code block; it does not normalize or trim arbitrary differences.

All **1,157 native tests pass**, with two existing ignored Linux notification
fixtures requiring an isolated D-Bus session. Strict workspace Clippy, Rust
formatting, full workspace tests and `dune build -j2 @all @runtest @fmt` pass.
The actual macOS editor/document tests pass, including native NSView text-client
coverage. Workspace tests took 242.115 seconds; Dune took 397.650 seconds; the
scoped desktop suite took 49.499 seconds.

The rebuilt public gallery passes a real macOS accessibility/keyboard probe:
`AXSelectedTextRange` is `[0, 401]` UTF-16 units and `AXSelectedText` contains 397
Unicode scalar values, with the literal tags appearing once. Native Copy contains
396 scalar values; the sole difference is the expected terminal logical newline.
The probe checks exact text, the range length, the fixed final code-block content,
rich reading order and native Copy. The application closes normally with exit 0;
all captured readable clipboard representations are restored and verified.
Executable SHA-256:
`15837b772602a3f1e012b2014829a8b5ab2f4bde908ac71fba47d6dc35595419`.

Both maintained forks reconstruct exactly from their pinned archives, excluding
Cargo.lock: GPUI **158 files**, Base **243 files**. Patch SHA-256 values:

- GPUI: `d66561a61c1ec5ec493b35271abd412decf8aecface9e9629b86574c6d2bfa2b`
- Base: `b9abb4e83748ec62c5369620f60e235bb22ce81c3502001055dd0b02560f667c`

Diff checks and the example documentation inventory pass (429 sources / 266
reviewed groups / 0 pending). The [report archive](rendered-accessible-selection-och17/reports.tar.gz)
and [manifest](rendered-accessible-selection-och17/manifest.json) retain commands,
source hashes, reconstruction results, original failures and the actual OS probe.
The earlier failed OS probe terminated its owned child after the assertion; only
the final successful run qualifies normal close. Neither probe used VoiceOver.
These local checks establish no current-source Linux acceptance.

## Remaining work

The existing cached ViewElement path reuses paint data without replaying AX
nodes/listeners. Current native application code does not call that cache API,
but cached-scene AX support still needs its own regression and replay design.
Do not claim cached-tree acceptance from ordinary redraw tests or disable caching
as a substitute for that work.

OS SetTextSelection still needs current final-tree authorization, native owner
and interaction-epoch checks, and deliberate focus/reveal behavior. Last-prepaint
coordinate mappings are not action authorization. Visual-line adjacency, actual
VoiceOver, consolidated platform/catalog/performance/resource qualification and
release distribution remain required. OCH-17/OCH-41 remain In Progress.
