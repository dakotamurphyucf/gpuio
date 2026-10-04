# OCH-41 password input validation

Local worktree, 2026-10-01, macOS arm64. This is an in-progress checkpoint,
not completed editor catalog or macOS release acceptance. No OS window is opened
by the checks below. The [contract](../design/plain-input-extensions.md#password-policy--implemented-contract)
defines display, clipboard, accessibility and plaintext observation behavior.

Commands run through the repository's isolated environment with `GPUIO_JOBS=2`:

| Check | Current evidence |
| --- | --- |
| `./scripts/gpuio exec dune build -j2 @test/view_api/runtest` | Pass: invalid multiline/password combinations, unchanged legacy editor bytes, independent Op73 fixture, retained node and privacy-only reconciliation, unchanged renders silent. |
| `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --test editor_privacy` | Pass: exact paired bytes, all truncated prefixes, trailing data and invalid privacy tags. |
| `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --test editor --test choice_picker` | Pass: three editor tests and nine picker tests. New tests cover single-line admission, failed-batch rollback, stable resource accounting and rejection of password policies on retained picker queries. |
| `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib editor_privacy_test` | Pass: same editor/focus and complete snapshot across hide/reveal/plain; undo/redo and marked composition retained; hidden Copy/Cut suppressed; revealed Copy/Cut allowed with undo recovery; rendered PasswordInput AX role and omitted value; Plain restores AX value. |

The native fixture mounts the production host View and editor with real session
transactions, an in-memory platform clipboard and an active TestPlatform
accessibility tree. Actions are dispatched through GPUI's installed handlers.
It does not validate physical keyboard events, AppKit secure-field projection,
screen-reader speech, OS clipboard ownership, IME candidate windows or GPU pixels.

The public `Password_preview` uses documented Eio/Bonsai/Core APIs and synthetic
text. Submission reports only a completion flag and does not print the password.
The full OCaml `dune build -j2 @runtest @fmt examples/gallery/main.exe` passes.
The full native `--features native-image-tests --lib` suite passes **531 tests**,
with two existing skips. Strict Clippy for native/protocol with all targets and
`native-image-tests` passes with `-D warnings`. A final focused Core run plus
`@fmt` passes after adding the initial-hidden-policy assertion. Catalog source
audit and `git diff --check` pass. Visual review and installed-consumer runtime
remain open. Required final Linux
checks, macOS acceptance and the unrelated black-startup issue remain milestone
requirements.

Detailed local logs and running handles are recorded separately in
`scratch/agents/root-20260929-m7-resumed/OCH-41-editor.md`; scratch is not a build
dependency. No commit, hosted CI or publication is claimed for this checkpoint.
