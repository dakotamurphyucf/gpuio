# A picker returns a path; Eio performs the file read

[main.ml](main.ml) demonstrates native open/save/capability queries and explicit
bounded text reading. Read error_text/display_path, component, then launch and three
diagnostic functions. [README](README.md) contains exact build/run commands;
[dune](dune) links Core/GPUIO/Bonsai/Eio and PPX. No service/assets are needed, but
open requires a file chosen by the user. See [isolated
setup](../../docs/development.md) and current [platform
scope](../../docs/platform-release-policy.md); picker support must be queried for
actual backend, not inferred from compilation.

`component env window graph` allocates content instruction once with `B.state`.
Bonsai graph is persistent; `let%arr` reads reactive content/setter to derive views
and effects. `and` lists dependencies, not threads. Config constructors validate
absolute /tmp directory hint and suggested notes.txt name. The hint is explicit
because Eio cwd capability can expose dot rather than an absolute native path.
`error_text` formats typed dialog errors; `display_path` displays valid UTF-8 bytes
directly and escapes invalid ones. Path bytes are not filesystem capabilities.

open_text uses `E.Let_syntax`/`let%bind` to evaluate `Dialog.open_` and await result.
Error updates content; Ok None is ordinary user cancellation and returns Ignore.
Exactly one path starts a window-scoped Eio task with explicit env fs capability.
`Eio.Path.with_open_in` and `Buf_read.take_all` bound buffer at 65,537 bytes;
successful text over 65,536 reports exceeds 64 KiB, valid UTF-8 without NUL displays
content, otherwise reports invalid text. Oversized read can itself fail the buffer
limit, reported as generic producer error; it is not a streaming viewer. Unexpected
selection cardinality reports error. Producer result updates state on UI loop; window
close cancels work/suppresses queued completion.

choose_destination awaits `Dialog.save` and displays selected path with “not
written”. It does not create/reserve/write destination. check_support probes exact
window backend without showing picker, formats multiple Files/Directories/Mixed and
save support; a successful capability snapshot does not guarantee later picker
success. One pending dialog/capability query per window; overlap Busy, pre-open
Not_ready, close/shutdown Closed. See [File_dialog](../../lib/eio/file_dialog.mli).

Click Open text file: native button action invokes OCaml effect, native picker
presents and returns correlated selection asynchronously; explicit Eio task opens
that path using fs capability and reads bounded bytes; result effect updates content,
Bonsai derives text, GPUIO submits native view. Native picker acceptance and path
selection are distinct from file read success and physical display. Native Rust owns
picker/platform callbacks; application owns content/I/O, not a native file handle.
Cancelling picker leaves prior content unchanged.

`App.run` owns GPUI OS thread and one OCaml Eio UI domain for component/effects,
opening a 720 × 480 window normally. Ordinary view has no save-writing action or
Close button; close the native window normally. No latest-request guard exists for
overlapping **file reads** after picker returns: an earlier slow read could overwrite
a newer result. Preserve a request identity if adapting to larger asynchronous loads.
[Scope](../../lib/eio/scope.mli) and [App](../../lib/eio/app.mli) define cleanup.

## Optional diagnostics

From repository root after building:

```sh
_build/default/examples/file_dialogs/main.exe --capabilities-self-test
_build/default/examples/file_dialogs/main.exe --self-test
```

capabilities_self_test opens two minimal windows, probes before native opening,
retries only Not_ready with 5 ms clock waits, and checks supported multiple-file/save
capabilities plus Busy and pending/after-close/shutdown Closed. It uses 15-second
timeout and no picker/paint readiness dependency. Success prints
GPUIO_FILE_DIALOG_CAPABILITIES_OK.

Ordinary `--self-test` instead opens two full components, captures pre-open picker
error, waits native render callbacks through promises with retries, opens one picker
then overlapping request, closes owner after 300 ms, checks both close replies,
starts second picker and shuts down after 300 ms. Task failures are collected, runner
errors asserted; success prints GPUIO_FILE_DIALOG_PUBLIC_OK. It tests native picker
cancellation/error delivery, not manual selected file/read or save writing.

read_self_test DIRECTORY is a harness-only branch invoked by `--read-self-test` with
one directory argument. It opens a window, awaits render, opens picker with accept
label Read test file, expects exactly DIRECTORY/LICENSE, explicitly reads via Eio and
checks Apache License text/size, then shuts down. A real driver must select file; see
README. Success prints GPUIO_FILE_DIALOG_READ_OK. `observe` and
`E.Expert.handle`/promise refs are diagnostic integration, not ordinary UI state. All
tests have bounded Eio timeouts; native callbacks do not prove physical display,
VoiceOver or Linux GUI acceptance. The macOS Python driver requires Accessibility and
reaps only its owned child.

To support actual save, keep `Dialog.save` as destination selection, then perform
explicit scoped Eio write with application overwrite/error policy. To load more than
64 KiB, choose bounded streaming/document ownership rather than removing all limits.
Handle path bytes separately from text UTF-8, cancellation separately from error, and
stale asynchronous results by identity. Keep file I/O outside `let%arr` and avoid
treating a selected path as granted authority.