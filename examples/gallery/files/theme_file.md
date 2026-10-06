# Bounded theme-file I/O and its expect test

[`Theme_file.load`](theme_file.ml) is the narrow Eio adapter for the gallery's
theme importer. Its [interface](theme_file.mli) accepts an Eio path capability
and returns a validated [Theme_profile](../model/theme_profile.md) or an error.
It has no Bonsai state or native view and does not decide whether a result is
still relevant to the selected window appearance.

Run the [gallery theme card](../theme_preview.md) for the interactive example.
The [application](../application.md) converts a native `File_path.t` using the
explicit environment filesystem capability before calling this adapter. The
preview starts it in a child task scope, so filesystem work is separate from
reactive view construction.

`Eio.Path.with_open_in` bounds the lifetime of the file handle. `Buf_read.parse`
uses `take_all` with a maximum buffer of `Profile.maximum_bytes + 1`. That extra
byte lets the pure decoder distinguish a file exceeding its 16 KiB limit;
larger reads can fail at the bounded reader. This is a small-profile reader,
not a streaming importer for arbitrary-size files.

Reader/decoder failures become `Or_error` values. The exception handler catches
expected `Eio.Io` failures. It deliberately does **not** catch Eio cancellation
or every unexpected exception and reinterpret them as malformed profile data.
Task cancellation must continue through the caller's structured lifetime.

On success, the caller receives an immutable profile. It must still check scope
activity and its request token before publishing a UI change. File-handle cleanup
does not by itself prevent a stale result from overwriting a newer user choice.
The [preview walkthrough](../theme_preview.md) follows that complete sequence.

## Read and run the test

[`test/theme_file_test.ml`](test/theme_file_test.ml) is an expect test owned by
this adapter; [its Dune stanza](test/dune) copies the Aurora input and declares it
as a test dependency. The library [Dune stanza](dune) keeps Core/Eio/model
dependencies separate from the gallery window.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @examples/gallery/files/test/runtest
```

The test opens the checked-in sample, writes padded fixtures at 16,383, 16,384,
16,385 and 32,768 bytes, and observes acceptance only at or below 16 KiB. It also
rejects malformed/missing files and confirms a pre-cancelled read never returns
an ordinary value. Generated files stay in a temporary test-directory subtree
and `Exn.protect` removes it. The declared fixture uses the filesystem capability
because Dune may symlink read-only inputs outside its confined test cwd.

This is real filesystem/cancellation evidence, not a native picker or GUI test.
To add another failure case, extend the fixture matrix or assertions without
promoting changed output blindly. To support larger profiles, change the single
model bound intentionally and retain exact-boundary tests; do not introduce an
unbounded `read_all` before validation.
