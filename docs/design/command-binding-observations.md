# Native command-binding observations

OCH-41 implementation contract. The native observer, paired transport, Core View
callback, Eio delivery and Bonsai value adapter are implemented. A focused local
macOS test passes actual widget-keymap lookup, composition-driven changes without
a retained-tree update, configuration/epoch replacement, coalescing, visibility
recovery and retirement. The public gallery now demonstrates the observer and
passes scoped macOS input/lifecycle checks in both the repository and a fresh
installed-library consumer. The expanded native context, row, workload and window
matrices also pass. Together with the typed-display evidence, this establishes a
local functional equivalent for the Kbd source row. Whole-catalog/release gates
remain open.

## Public interface

[Command_binding](../../lib/core/command_binding.mli) describes queries and samples.
Mount `View.command_binding_scope ~config ~on_update children` to receive them.
The owner is an ordinary container with no extra keyboard-focus stop. Constructing
a config alone does not query a window, register a shortcut or invoke an action.

Queries contain 1..64 unique ordered typed targets:

- `Command of Command.Id.t` resolves an application command registry.
- `Native_action of Command.Native.t` looks up one of the six native edit actions
  (Copy, Cut, Paste, Select_all, Undo, Redo) in GPUI's actual keymap.

Contexts have distinct meanings:

- `Context.focused` samples the window's logical focus and uses the same registry
  root fallback as shortcut invocation. It applies the known routing gates.
- `Context.here` describes registry declarations at the observer's native tree
  position. It accepts registry targets only, with Declared dispositions.
- `Context.editor snapshot` keeps only the exact editor window/node lease, not its
  draft, selection or revision. Another window is rejected during preparation
  and native admission. A removed editor yields Context_gone and never targets a
  replacement generation; a hidden/unrendered retained editor yields Suspended.
  Results describe that editor's hypothetical focus context, with Declared
  dispositions. This is not an instruction to focus it.
- `Context.native_context "Input mode=visible"` describes native context facts,
  not a keybinding predicate. It accepts native-action targets only. Malformed
  facts yield Invalid_context; valid queries report Declared bindings.

Ready entries preserve the target order and distinguish missing registry commands,
present commands with no shortcuts, native unbound actions, native sequences and
unsupported native representations. Samples carry positive epochs within one
mounted configuration. They are historical observations, not a lock on future
focus or a promise that the OS/widget will deliver a shortcut.

`Gpuio_bonsai.Command_binding.component ~config ~f graph` supplies the latest
observation (initially None) to `f`, which builds the children. Only observation
state belongs to the active configuration visit. Config changes and reactivation
return to pending; delayed effects from retired visits cannot restore old data.
Child computations retain their state across query changes. Mount the returned
view for the component's active lifetime. A dynamic optional style value and a
stable optional key control its ordinary container.

`Presentation.Kbd.create` displays declared single-chord shortcuts.
`Presentation.Kbd.of_native_stroke` displays observed native strokes, including
Function and native key names outside the registration domain. It shares the
filled/outline/plain styles and localized accessible-name validation. Render all
strokes of a sequence in order; the display helper does not register them.

## Registry resolution shared with invocation

`Tree.commands_from` enumerates ancestor registries nearest first, preserving
within-scope declaration order. It deduplicates IDs **before** filtering shortcuts,
input policy, enabled state or priority. Thus disabled/unbound/differently phased
inner definitions still shadow the same outer ID; siblings do not participate.
The observer's bounded traversal shares this raw declaration source, counting
both ancestor visits and shadowed declarations against its work budget.

`command.rs` shares `InputContext`, physical modifier conversion and GPUI's own
key matcher between invocation and inspection. Primary and Super/Control aliases
agree with the display API. Modified_only preserves ordinary editor typing;
composition and Never/Always text-input policies remain native. Native-first
Tab/Shift-Tab and ordinary button Enter/Space stay reserved. Invocation preserves
its early navigation fast path.

The first matching declaration reserves its chord within a phase even if disabled,
blocked by a modal scope or unavailable for the retained native edit target.
Failure does not fall through to a later command in that same phase. A failed
Override does not consume the event, so native handling and NativeFirst may follow;
a successful Override wins before NativeFirst regardless of declaration order.

For Focused registry results, candidate dispositions report Override,
Native_first or an explicit suppression reason: disabled, blocked scope,
unavailable native edit action, composition, text-input policy, native navigation
or a conflicting command. Native_first describes eligibility if the event reaches
that phase. An eligible Override belonging to the same command is represented by
its own candidate rather than a self-conflict. Candidate order remains declaration
order; no query executes a command as a probe. Here/Editor results preserve enabled
metadata but do not borrow unrelated current-editor gates.

## Native widget bindings are a separate source

A native `Command.native` with no declared shortcut can coexist with a working
widget binding. Missing registry declarations are not evidence of native unbound
actions. Native queries use GPUI's last-applicable-binding precedence, distinct
from the registry's nearest/first ordering.

After paint, Focused inspection explicitly obtains `window.focused(cx)` and calls
`highest_precedence_binding_for_action_in` with that handle. The unqualified
`highest_precedence_binding_for_action` reads the dispatch tree's transient context
stack and returned no Input Copy binding in our post-paint test; do not infer the
focused context from it there. Editor leases use their actual native focus handle.
Native_context uses `highest_precedence_binding_for_action_in_context`.

Focused native results additionally check listener availability and a consuming
application Override for the first stroke. Widget disposition means an applicable
native keymap route, not a guarantee about widget-local preconditions, clipboard
contents, OS-reserved events or future focus. Registry-native commands can target
a retained editor after toolbar focus; focused widget queries describe current
focus instead. These are intentionally different contracts.

Up to eight complete ordered native strokes preserve physical Control, Alt,
Shift, platform and Function modifiers. Invalid native strokes or longer sequences
produce explicit Native_unsupported results. There is no lossy conversion back to
GPUIO's single-chord Shortcut registration type.

Native context parsing is iterative and bounded to 1024 input bytes. It accepts
Unicode alphanumeric/underscore/hyphen identifiers and nonempty key=value facts,
with whitespace and GPUI's first-definition-wins behavior, using KeyContext add/set.
The pinned `KeyContext::parse_expr` recursively retries unsupported punctuation
without consuming it; arbitrary query text is never passed to that parser. No
upstream fork change is needed.

Pinned sources: `vendor/gpui/src/window.rs`, `vendor/gpui/src/keymap/context.rs`,
`vendor/gpui/src/key_dispatch.rs` and
`vendor/gpui-base/src/input/base/state.rs`.

## Lifetime, bounds and delivery

A query owns its window/node/handler generations. Config changes require a fresh
handler during atomic native admission and reset the producer epoch. Core rejects
retired owners, mismatched result provenance, future tree revisions and duplicate
or out-of-order epochs. Preparing a replacement alone does not mutate accepted
callback/epoch state. Callback-only changes preserve the native subscription and
use the latest accepted closure. Removing observer metadata can retain its ordinary
container and children.

Sampling runs after native paint. Relevant editor/focus/registry changes can cause
new samples, including child-only editor composition changes. Unchanged results
are silent even when tree revision changes or a frame is forced. No query timer,
request-next-frame loop, synchronous OCaml call or command invocation is introduced.
Hidden/inactive owners report Suspended and recover when rendered again. Epoch
exhaustion emits the terminal Epoch_exhausted state at Int64.max_value; it never
wraps. Reconfiguration or remount supplies a new lifetime.

Each window admits at most 64 indexed owners. One sampling pass has 65,536 units
of registry traversal/candidate work; repeated context walks and chord winners
share per-pass caches. Budget exhaustion reports Capacity, never false absence.
GPUI's own native keymap lookup remains native work. Changed native state or query
configuration can make a later frame fit the budget; no idle retry timer is added.

The mailbox has a separate latest-value class. One undrained result per owner is
replaced by newer results, without consuming or evicting lossless command-input
capacity. There is no cross-class chronological guarantee for these replaceable
snapshots. Config changes/unmount prune old owner/handler output; window destruction,
close and transport shutdown release pending query samples. Producer admission
reserves two maximum-size observations (cached and queued) per owner in the existing
retained-tree/session quota. This is a conservative reservation, not measured RSS.
The separate mailbox class is additionally bounded by half the 256 MiB session
quota and at most 32 windows times 64 owners. Typical small results allocate their
actual data size. Query metadata and results retain no editor draft.

## Paired transport

Capability bit 52 is `CAP_COMMAND_BINDINGS`; the current complete mask is
`9007199254740991`. The independently specified Hello bytes are:

- Only binding observations: `0001fc0000000000001000`.
- Current complete mask: `0001fcffffffffffff1f00`.

Operation tag 62 is `Set_command_binding (node, config option)`; None clears it.
Event tag 66 is `Command_binding_observed (window, node, handler, tree_revision,
observation)`. The setter applies to an ordinary Container; it adds no Kind tag.
Existing command types are shared through Command_wire without changing their
serialization. Old capability masks and previous message tags remain unchanged.

Context tags are Focused 0, Here 1, Editor 2, Native_context 3. Target tags are
Command 0 and Native_action 1. Entry tags are Missing_command 0, Registry 1,
Native_unbound 2, Native_binding 3 and Native_unsupported 4. State tags are Ready 0,
Suspended 1, Context_gone 2, Invalid_context 3, Epoch_exhausted 4 and Capacity 5.

Configurations are at most 32 KiB; observations at most 256 KiB. There are at most
four candidates per registry command, eight strokes per native sequence, 256
bytes per native key/command ID and 1024 bytes per context. Physical modifier bits
are Control 1, Alt 2, Shift 4, platform 8 and Function 16. Both codecs bound nested
allocation before admission. Core validates ordered result provenance against the
specific query before dispatching the callback.

## Evidence and remaining acceptance

Local macOS 14.5 arm64, 2026-09-30:

- The earlier shared-routing checkpoint passes native command-controls regressions
  for shadowing, phase precedence, disabled conflicts, reorder and disposal.
- Seven Core binding tests pass paired data/envelope bytes, malformed payloads,
  lease/target domains, handler/config/epoch fencing, latest callbacks, preparation
  isolation, retained children and observer removal.
- The native unit suite passes 420 tests (two ignored), plus three binding
  admission/session/work-budget tests, three registry-resolution tests and the
  existing command-session integration test.
- The actual-window `native_command_binding` fixture passes native Copy lookup,
  registry declarations, malformed/valid native facts, exact editor leases,
  child-driven marked/committed text changes without retained-tree revision or
  parent refresh, configuration epochs, coalescing, hidden/revealed owners,
  removed-editor status, silent unchanged frames/idle and undrained-owner cleanup.
  Its macOS text-client calls are controlled test input, not physical keyboard,
  real OS IME candidate-window or VoiceOver qualification.
- The Bonsai value adapter passes configuration changes, A-to-B-to-A revisits,
  component deactivation/reactivation, stale-effect fencing and preserved child
  state with both optimization settings.
- Full Dune build, expect tests and OCaml formatting pass. All 252 Rust protocol
  tests pass, including independent setter/event/current-capability bytes. The
  final native non-GUI run passes 433 tests (two ignored), including six session
  negotiation/queue/lifecycle checks. Strict native Clippy with all test targets
  and `native-tests`, strict protocol Clippy with all targets, and workspace Rust
  formatting, pass.

Commands for this mounted-observer checkpoint (repository isolated toolchain):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --locked
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --lib --test command_binding --test command_resolution --test commands --test session --locked
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native --all-targets --features native-tests --locked -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-protocol --all-targets --locked -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-tests --test native_command_binding --no-run --locked --message-format=json
```

The local native runner executes the `native_command_binding` compiler artifact
from the final build under a 120-second process-group watchdog with termination,
five-second kill escalation and reaping. The equivalent hosted execution is
checked into the CI workflow with a 90-second bound after compilation.

The native test uses a bounded process-group watchdog and closes its window on
success or assertion failure. Required macOS CI now includes it; hosted results
are not implied by these local checks.

Consolidated catalog acceptance and all OCH-17 macOS release gates remain required. Linux build/unit/
private-bus/consumer checks remain required; full Linux desktop qualification is
still deferred to OCH-47. The local Kbd source mapping does not complete OCH-41 or
milestone 07.

### Expanded native matrix — 2026-09-30

The same `native_command_binding` target now additionally passes:

- A nearer disabled or unbound command shadows the same outer ID. A different
  disabled/enabled command reserves its chord within a phase. A failed Override
  permits NativeFirst; a successful competing Override suppresses it. Post-paint
  observations are checked against the production dispatcher on the same tree.
- A trapping native focus scope moves focus to its button, changes the effective
  registry path, and restores the original editor/conflict on removal. The editor
  retains its focus handle throughout registry insertion/removal and modal use.
- A sparse 1,000-row managed list renders observers in the first and last rows.
  Scrolling suspends the unrendered row and recovers its original subscription
  with a higher epoch. Eviction prunes queued observations before delivery.
- A valid registry of 1,024 commands with four distinct shortcuts each and a
  64-target query reaches the work limit and reports Capacity. Forced unchanged
  frames remain silent. Reducing the registry recovers Ready, with true missing
  entries distinguished from the surviving command's four candidates.
- Two real windows share one session/mailbox and reuse identical node/handler IDs.
  Their values and epochs remain independent. Closing a window prunes its queued
  samples without losing the survivor's; reopening its slot with a new window
  generation starts a fresh epoch and does not revive old output.

All four fixture markers pass and the process exits successfully under the local
120-second watchdog. Auxiliary windows open without requesting foreground focus;
the first fixture window exercises actual GPUI focus. Dispatcher calls are native
test injection, not additional physical OS-keyboard evidence. The public gallery
provides the separate OS shortcut/clipboard checks below. No production fix or
dependency/fork change was needed for these matrices. Final strict native Clippy
(`--all-targets --features native-tests --locked -- -D warnings`), workspace
rustfmt and the structural catalog audit pass. The previously passing full Dune,
protocol and native unit results remain separate evidence for unchanged code;
this checkpoint adds native fixture coverage and documentation only.

## Public gallery

Component Studio's **Shortcuts in context** card uses public APIs only. The
editor controller is outside the query branch, so changing the context or hiding
the observer does not remove the editor. It demonstrates Focused, Editor, Here
and Native_context queries, independent registration/enabling, chord replacement,
macOS/Linux label formatting, invalid facts and observation retirement.
The preview renders every returned native stroke, and keeps query state separate
from the command invocation counter.

The focused macOS driver passes 20 binding/name/identity cases in both the
repository app and a fresh installed-library consumer: live widget Copy
lookup, actual OS Command-C clipboard behavior, registered/unregistered/disabled
and replacement-chord invocation, hypothetical editor lookup while a button is
focused, declaration/native-only target sets, invalid-context recovery and epoch
reset, silent unchanged epochs, query hide/remount, both application themes and
page departure/remount. Callback counts distinguish inspection from invocation.
The clipboard is restored; the window exits successfully and is reaped.
The independent consumer stages packages under its own prefix and builds with a
separate backend lockfile without installing into any opam switch. It reuses the
local toolchain/native sources; this is not clean-machine distribution evidence.
Both runs report `GALLERY_BINDING_OBSERVATIONS_OK` and
`GPUIO_GALLERY_AX_OK: section=binding-observations`. Full Dune build/tests/format,
Python syntax and the structural catalog audit also pass. The dark screenshot
was inspected; this is not a GPU pixel assertion, OS IME or VoiceOver qualification.

```sh
python3 scripts/test_gallery.py --section binding-observations
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-m7-binding-consumer-20260930
python3 scripts/test_gallery.py --section binding-observations --executable /private/tmp/gpuio-m7-binding-consumer-20260930/consumer/_build/default/main.exe
```

The focused section is included in `core` and `all`. A fresh combined gallery run,
hosted CI and the release gates remain separate acceptance work.
