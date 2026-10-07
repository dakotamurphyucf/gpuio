# Rendered text projection foundation — OCH-17 / OCH-41

Bounded Markdown/HTML preparation now produces an immutable logical rendered-text
map alongside the native AST. It includes the same structural separators and
object alternatives as native plain-text Copy, with weak references to the
existing selection owners. Equal text in another preparation has a different
identity. Checked positions use UTF-8 scalar boundaries and reject the middle of
a CRLF break; they are not OS UTF-16 offsets.

This is the first implementation stage of the
[rendered selection plan](../design/rendered-document-selection.md).
**Accessible selection publication and mutation remain unimplemented.** The
projection's local fragment query does not reconstruct directed document
selection, virtual cross-view selection or Select All. Those require integration
with the existing selection controller. No new OCaml API or synchronous callback
is introduced.

## Ownership and admission

Changing the copy format keeps the installed projection identity. Replacing the
prepared document installs a new identity; ordinary unbounded parser updates
clear the bounded projection. Holding its immutable text does not retain native
selection owners after unmount.

Preparation bounds logical copy text to 128 KiB and provenance parts to 16,384.
Rich-document workers reserve a conservative maximum before parsing and reduce
the reservation to retained text/vector capacity after preparation. Code and
diff workers do not reserve this unused rich-text allowance. The retained result
owns its resource charge until release. These are admission units, not measured
RSS or performance acceptance. Oversized preparation follows the existing source
fallback.

## Validation

Local macOS 14.5 / arm64 / Apple M1 Max:

- Native unit tests: 1,081 pass, two existing ignored. Cases cover native copy
  structure, repeated Unicode, joined emoji, combining marks, CRLF, tables,
  custom alternatives, empty text and all 200 blocks of an unpainted document.
  Replacement, copy-format identity, weak-owner cleanup and pool reservation
  reduction also pass.
- Strict native all-target Clippy and Rust formatting pass.
- Full Dune `@all @runtest @fmt` passes, both before and after the shutdown
  callback guard described below.
- The actual native document suite passes selection policy, style, mixed-view
  Copy, streaming publication, GPU highlighting and owner disposal.
- The final public-gallery probe passes rendered Select All/Copy, rich reading
  order and normal close with Rust backtraces enabled. As expected at this stage,
  `AXSelectedText` and `AXSelectedTextRange` remain absent.
- Reconstructing Base from its pinned source archive and updated patch matches
  the vendored files (excluding the ignored local Cargo.lock).
- The example documentation inventory remains 429 sources / 266 reviewed groups /
  zero pending; no example API changed.

The initial compilation failure exposed an inaccessible internal owner field;
its visibility was narrowed to the containing text module. Two initial fixture
failures assumed the HTML image adapter produced an inline object. Inspection
showed that it produces a block; separate tests now cover its actual native copy
separators and a true Markdown inline object. The first full suite then exposed
an exact resource-reservation assertion that lacked the new projection allowance.
Only that initial expected reservation was adjusted; the existing retained-size
reduction and release checks remain intact.

## Shutdown regression found during validation

The first real gallery probe copied the expected text and preserved rich reading
order, then crashed while closing with `RUST_BACKTRACE=1`. The macOS crash stack
shows `libunwind::CFI_Parser::decodeFDE` reached through Rust backtrace capture,
`WeakEntity<TextViewState>::update`, and a deferred native selection clear handler
during `App::shutdown`. That update constructs an error when the owner is already
released; the caller ignored it. The same binary closed normally in a diagnostic
control with both Rust backtrace variables disabled. This isolates an optional
diagnostic path; it is not evidence that the underlying mixed-stack unwinder is
generally safe.

The TextView selection adapter now upgrades its weak owner before dispatching
selection, auto-scroll or clear callbacks. Retired owners produce an ordinary
no-op without constructing the ignored error. The window selection cache is still
cleared. This preserves weak ownership and changes no OCaml exception policy.
The permanent `scripts/test_macos_document_shutdown.py` regression deliberately
enables both Rust backtrace variables in its owned child, copies a selected
document and requires a normal window close/exit. It is included in macOS CI.
The initial crash and diagnostic control remain part of the evidence; disabling
backtraces is not the fix.

After the guard, both the original probe with `RUST_BACKTRACE=1` and the permanent
regression with `RUST_BACKTRACE=1 RUST_LIB_BACKTRACE=1` pass on the same rebuilt
gallery. Both child applications exit normally and the full captured clipboard
is restored. No VoiceOver or desktop settings changed. The full native suite,
strict lint, Dune checks and patch reconstruction pass again after this change.

## Reproduction and retained evidence

Use `GPUIO_JOBS=2` and the isolated wrapper:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-canvas-tests,native-image-tests,presentation-diagnostics -- -D warnings
./scripts/gpuio exec dune build -j2 @all @runtest @fmt
./scripts/gpuio exec cargo fmt --all --check
python3 scripts/test_macos_document_shutdown.py --log scratch/document-shutdown.log --report scratch/document-shutdown.json
```

The native document target is `native_highlight_document` with
`native-image-tests`. The archived driver bounds its child process and restores
the captured clipboard after exit. The [archive](rendered-text-projection-och17/reports.tar.gz)
and [verified manifest](rendered-text-projection-och17/manifest.json) include exact
commands, before/after results, the normalized crash stack, diagnostic control,
source patch and source/binary hashes. Initial failures are preserved. All local
build and test children have exited.

The native semantic hierarchy, guarded selection actions, root/fresh-consumer
AX qualification, actual VoiceOver behavior, current-source hosted/Linux checks
and broader release acceptance remain open.
