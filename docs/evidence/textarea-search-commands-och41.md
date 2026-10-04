# OCH-41 programmatic text-area search commands

2026-10-01, local macOS arm64, isolated stock OCaml 5.3/Bonsai v0.17 and pinned
native dependencies. Uncommitted checkpoint; this is not catalog completion or
physical desktop acceptance.

`Text_input.Config ?searchable:true` opts ordinary text areas into search. Op80
admission is multiline-only and atomic; retained configuration changes preserve
the editor. Disabling closes search without replacing the draft. Eio
`Text_input.search_command` routes typed open/close/query/navigation/read and
revision-checked replacements through the existing correlated request mechanism.
Metadata replies do not update text. Replacement replies feed normal monotonic
controller observations, so delayed replies cannot erase newer typing.

Editor command10, result5/6 and appended error11..13 variants have independent
paired fixtures. Rust decoding bounds query length before allocating its string.
OCaml reply validation checks metadata and replacement text revision/length
agreement; the request adapter also validates response kind, owner and counts.
The native adapter checks current editor/search revisions before replacement,
composition, editability, bounds, focus gating and retained identity. Zero matches
produce a count-zero acknowledgement without editing.

Three new native tests exercise the real retained editor on TestPlatform:
configuration admission/atomic rollback, checked replacement and stale text or
navigation, Unicode undo, disable/re-enable retention, stale retained entities,
composition/read-only/disabled behavior, expansion limits and zero-match replies.
The complete native library suite passes **573 tests, with two existing skips**.
The admission fixture initially used a noncontiguous node slot, omitted the
required input handler and attempted a forbidden editor `SetText`; it was corrected
to a valid editor and a preceding style change before testing rollback. No
production admission rule was weakened.

The full protocol suite passes **311 tests**. The final targeted `editor_search`
suite also passes both tests, including request/result/config fixtures,
truncated/trailing data and invalid query/stamp rejection. Core expect tests cover
metadata, exact ownership, multiline opt-in, retained reconciliation and paired
reply validation. New Eio checks cover correlation, cross-editor stamps, response
kinds, malformed counts, close cleanup and a delayed replacement reply after newer
native typing. Their full Dune run passed those tests but reported a formatting
difference in the new test dependency list; that list is corrected. The earlier
full build also required extending the chat demo's conservative close-error match
for the appended error variants.

The final full OCaml tests, OCaml/Rust formatting, gallery build and strict
Rust lint checks all pass at this checkpoint. Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`, with `-j2`:

- `cargo test -p gpuio-native --features native-image-tests --lib --offline`
- `cargo test -p gpuio-protocol --offline`
- `cargo test -p gpuio-protocol --test editor_search --offline`
- `dune build @runtest @fmt examples/gallery/main.exe`
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`

No vendor source changed in this checkpoint. The Base patch remains
`3592b40ae45c0ed8dd170b0c02afaf48426136930f5ea09f74636fb655068b8c`, whose preceding
reconstruction matched all 233 files. No OS windows were opened by these tests.

Remaining: coalesced native search-change events, reusable Bonsai search bar,
query focus and restoration, Find/F3/Escape integration, gallery interaction,
physical macOS keyboard/IME/accessibility and resource qualification. Required
Linux automated checks remain separate; Linux desktop acceptance is deferred.
See [the command contract](../design/textarea-search.md).
