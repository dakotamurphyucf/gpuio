# OCH-10 native editor validation

Local validation: 2026-09-13, macOS 14.5 arm64, stock OCaml 5.3.0,
Bonsai v0.17, Dune 3.24.2 and Rust 1.97.1. This records local evidence;
required hosted macOS/Linux checks and merge are still pending.

| Check | Evidence |
| --- | --- |
| `./scripts/gpuio test` | Pass: OCaml expect/runtime/codec suite and all workspace Rust tests/doc tests |
| `./scripts/gpuio exec dune build @fmt @all` | Pass; additional public selection assertions built and passed `@fmt` |
| Workspace Clippy with `gpuio-native/native-tests`, all targets, `-D warnings` | Pass |
| `_build/default/examples/text_input/main.exe --self-test` | Pass: two public-API windows, reverse UTF-8 selection, undo/redo selection, revision guards, stale unmount and close |
| `cargo test --locked -p gpuio-native --features native-tests --test native_editor` through project environment | Pass: actual window and native input/accessibility callbacks |
| Pinned GPUI Base reconstruction | All 226 files match archive + manifest adaptation + verified patch byte-for-byte |

The native test covers joined-emoji deletion, clipboard copy/paste paths,
Tab/Shift+Tab, declared Enter/Shift+Enter behavior, composition suppression,
exact submitted revision/text, type-then-delete delayed-clear rejection, auto-grow
at 1/4/8 lines, resizing without resetting the session, read-only edits,
disabled focus/tab skipping, re-enable, stale close, and native accessibility
label/role/Focus/SetValue/focused-state queries. Native frames are synchronized
through GPUI callbacks instead of guessed delays before sending text.

The Mac composition scenario invokes the actual window's NSTextInputClient
marked/committed text methods, including UTF-16 selection coordinates. It checks
Japanese composition and undo/redo of the committed composition transaction.
Accessibility walks the window content view's actual AccessKit-backed native
subtree. These are native callback checks; physical IME candidate-panel behavior
and a comprehensive screen-reader audit are not claimed.

Paired OCaml/Rust fixtures independently agree on a 168-byte request stream and
173-byte event stream, including appended editor tags and capability negotiation.
The pre-existing bridge fixture remains unchanged. Unit checks cover malformed
snapshots, selections, bounded decoding, controller identity, callbacks, retained
state, tree validation/rollback and mailbox coalescing/backpressure.

Reuse evaluation originally passed 154 upstream input-engine tests. Three
additional engine regressions cover revisions/limits/composition, bounded pending
undo data and grapheme context across rope chunks. The original reuse probe is
not an assertion that the entire styled GPUI Component catalog is compatible.

Linux compilation/unit tests are required in CI. X11/Wayland editor GUI scenarios
are informational under the owner's development gate; Linux OS IME and complete
GUI acceptance remain OCH-17. No Linux GUI pass is inferred from local macOS tests.

Local detailed logs live in the ignored per-ticket notepad directory
`scratch/agents/root-20260912-milestones`; CI preserves corresponding logs as
workflow artifacts. The behavior contract and exact source pins are in
`docs/design/native-editor.md` and `third_party/sources.json`.
