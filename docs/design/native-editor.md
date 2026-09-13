# Native editor contract (OCH-10)

Use a native-owned editing session with a Bonsai controller observing snapshots.
An observation never writes text back. Initial text applies only when a native
node is created; later replacements are explicit commands. Single-line input and
multiline composer share these semantics, including while other windows or
streaming responses cause Bonsai recomputation.

## Reuse decision

Vendor the unstyled `gpui-base` crate from GPUI Kit commit
`84f57fdfcb4910623fb0bb7f795b077e249f9271` (0.6.1, Apache-2.0), retaining source,
license, the original manifest, and explicit adaptation patches. The current
published package uses a different GPUI package identity. The adapted manifest
uses our existing Zed GPUI commit `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`
for GPUI, macros and sum-tree. The OCaml/Rust toolchains and Bonsai pin stay fixed.

The standalone compatibility probe compiled without Rust source changes and
passed all 154 upstream `input::state::tests` against our GPUI pin. Those use
GPUI TestAppContext and are not OS IME acceptance. GPUI Base supplies rope-backed
editing, composition, undo/redo, clipboard, focus, auto-grow
and reusable control behavior. GPUIO adapts ordinary cursor/deletion movement to
extended grapheme boundaries (the upstream defaults use Unicode scalars).
GPUIO must supply revisioned bridge commands,
resource budgets, explicit submit policy, observation delivery and semantic
integration; successful reuse tests do not establish those additional contracts.

Source and manifest references: [upstream base crate](https://github.com/longbridge/gpui-kit/tree/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/base),
[workspace dependencies](https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/Cargo.toml).
Reconstruct deliberately with `scripts/vendor_gpui_base.py`; normal builds use the
committed snapshot and Cargo.lock. The source archive and patch hashes are pinned
in `third_party/sources.json`.

## Public interface

`Gpuio.Text_input` provides validated configuration, revision, selection,
snapshot, submission and command value types. `Gpuio_eio.Text_input.create`
receives the owning window and returns a Bonsai controller value. Its `view`
produces a typed view; `snapshot` is absent until the native editor mounts.
Explicit focus/select/replace/clear operations return typed response effects.
`clear_if_unchanged controller submission` uses the submitted revision, so delayed
send completion cannot erase subsequent typing. Implementation and local/hosted validation are recorded in
[the evidence report](../evidence/native-editor-och10.md) and
[PR #6](https://github.com/dakotamurphyucf/gpuio/pull/6).

The native node generation is the editing-session lease. Commands capture that
lease, not a name that silently resolves to a replacement. A controller has one
native placement; duplicate placement fails validation. Reordering a keyed editor
retains its native state. Unmount/close invalidates old commands, observations and
pending deliveries. Configuration changes do not replay initial or observed text.
Changing single-line/multiline mode creates a new native session.

Selections use UTF-8 byte boundaries with explicit anchor/head direction. OS
UTF-16 coordinates are converted inside Rust. Command ranges are validated before
mutation. Replacements specify selection policy and whether the edit is undoable
or resets history. A native edit counter advances synchronously on every accepted
text mutation, including composition and undo/redo; typing then deleting back to
the same text must still invalidate a stale conditional clear. Revision counters
never wrap.

Native declared key rules decide whether Enter submits or inserts a newline;
Shift+Enter inserts a newline in a chat composer. Composition suppresses submit.
A submit event captures exact native text and revision. No late OCaml callback
can cancel an already performed native default action.

Bounds apply to live text, undo history (including pending composition), editor
count, command count and queued event bytes. Snapshots can coalesce by editor;
submit and command-result boundaries retain their ordering. Closing/unmounting
releases subscriptions, input handlers, focus and cursor timers. Tests must
separate mock engine behavior, real native input callbacks, OS IME candidate UI,
and Linux build-only coverage.

## Implemented API and bounds

The public implementation is in `lib/core/text_input.mli` and
`lib/eio/text_input.mli`; the executable example is `examples/text_input`.
Configuration requires an accessible label and selects single-line or multiline
mode. It includes placeholder, read-only/disabled, submit-on-Enter, initial focus,
and minimum/maximum rows. Tab and Shift+Tab navigate controls. A disabled input
is excluded from tab navigation; explicit focus is a no-op and returns its current
snapshot. Programmatic replacement remains permitted for read-only/disabled
editors, while native user edits are rejected.

Text is limited to 256 KiB of valid UTF-8 without NUL. Single-line commands also
reject CR/LF. Selection offsets are scalar-aligned UTF-8 byte offsets; ordinary
keyboard movement/deletion uses extended grapheme boundaries. Explicit selection
and platform IME ranges may target scalar boundaries inside a grapheme.

Each editor reserves 8 MiB against the existing 64 MiB/window and 256 MiB/session
logical budgets. Undo history has a 2 MiB logical payload/selection budget,
including pending composition. Oversized composition transactions are discarded
from history as a whole so undo never replays only part of one. These budgets
bound retained logical data; they are not claims of exact allocator or RSS usage.
The OCaml runtime permits at most 64 pending editor commands. Native input output
is bounded by event count and 4 MiB of estimated encoded bytes; drains remain
within the 1 MiB envelope limit. Adjacent change observations can coalesce;
submissions and command results are ordering barriers.

The native counter is independent of retained-tree revisions and always advances
synchronously before observation delivery. Submit captures text, selection, focus
and edit revision at the native action. A command checks both the window/node
generation and any requested edit revision before changing text. Closing completes
pending commands with a typed error; unmount makes existing leases stale.

## Native adaptation and verification

`third_party/patches/gpui-base.patch` records the changes to the pinned base:
bounded text/history, synchronous revisions and submit payloads, grapheme boundary
traversal over rope chunks, disabled focus policy, and a Rust-only decorator on
the actual focus-owning editor element. The decorator lets GPUIO attach labels,
values, semantic state and accessibility actions without registering duplicate
tab stops on an outer wrapper. It does not transfer callbacks across the FFI.

The ordinary test command runs OCaml expect/codec/reconciliation tests and Rust
protocol/session/mailbox tests without opening application windows. The optional
`native-tests` feature builds an actual-window harness; run it locally for fast iteration and on macOS CI for the final gate.
It checks native NSTextInputClient marked/committed text, composition submission
suppression, undo/redo, graphemes, clipboard, Tab/Shift+Tab, auto-grow, disabled and
read-only behavior, revision races, and native accessibility focus/value actions.
The public example's `--self-test` exercises two windows through the OCaml API.
Linux compiles these tests and runs the non-Mac scenarios in the informational
X11/Wayland jobs. Full Linux GUI/IME acceptance remains OCH-17.

Direct NSTextInputClient and accessibility calls exercise actual native callback
paths. They do not automate a physical input-method candidate panel or constitute
a complete screen-reader audit. The linked evidence report records the hosted editor checks; completion also
requires the protected-branch merge. Local foreground GUI tests are authorized for fast iteration when focus is
necessary. Prefer background checks where valid and retain macOS CI validation.
