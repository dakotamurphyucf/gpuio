# Capability-based document reads and atomic snapshot replacement

[document_file.ml](document_file.ml) and [document_file.mli](document_file.mli)
provide synchronous-looking Eio load/save functions over explicit Path/Flow
capabilities. They own no Bonsai computation, native handle or mutable workspace.
The adjacent [expect tests](test/document_file_test.ml) share this walkthrough.
Read load, save and its protected cleanup, then the three tests.

From the repository root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/signal_studio/files -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

Use Save/Open in the app for the native panel path. Tests run nongraphical Eio
filesystem operations; they do not prove physical panel interaction or platform
GUI acceptance. Commands are source-reviewed, not newly executed here.

## Bounded reads and typed errors

load takes an Eio.Path.t, opens it with with_open_in and uses Buf_read.parse
with take_all and max_size 16385 (16 KiB plus one detection byte). Reader format
errors become Or_error; contents then pass [Workspace.decode](../model/workspace.md),
which rejects input over 16384 bytes and validates syntax/version/run/IDs/positions.
The scoped open closes its flow on completion/error. Expected Eio.Io exceptions
become Or_error.of_exn. Cancellation and other exceptions propagate rather than
being misreported as an invalid workspace. The result is an immutable Workspace.t;
this function does not install it or change current selections.

## Save one snapshot without following a destination symlink

save receives a path, an explicit random Flow.source and an already-valid workspace.
Path.split must produce a parent and filename; otherwise it returns an error.
with_open_dir holds that parent capability open throughout the operation. Sixteen
random bytes become 32 hex characters in a .gpuio-save- sibling name. Exclusive
creation with mode 0600 prevents overwriting a colliding temporary file; collisions
fail rather than retrying. Application supplies secure_random. The random bytes
choose a filename, not simulation data or document contents.

Inside with_open_out, Exn.protect writes Workspace.encode, syncs the open file,
then renames the sibling onto the destination within the held directory. Before
rename the old destination is untouched. Rename replaces a destination symlink
itself; it does not write through that link. The finally cleanup runs inside
Eio.Cancel.protect so cancellation does not interrupt unlink. Not_found is normal
after successful rename and is ignored; other cleanup errors remain errors.
The outer catch converts Eio.Io to Or_error and cancellation still propagates.

The operation makes a private replacement file, so existing permissions are not
preserved. Syncing the file does not promise crash durability of the containing
directory. A cancellation after rename can leave the new file installed even if
the caller does not accept a completion. The [document controller](../documents.md)
therefore treats reset as stale-result fencing, not a promise that disk writes
were cancelled. Its application Scope.start executes these functions away from
view construction, captures a submitted save snapshot and preserves edits made
while reading/saving. File success and native represented-file metadata are
separate results.

## What each expect test establishes

[test/dune](test/dune) enables ppx_expect inline tests and links Eio_main;
[files/dune](dune) links Core/Eio and the pure model. with_directory creates a
0700 signal-document-fixture under the test working directory inside Eio_main.run,
passes its secure_random capability and uses Exn.protect to remove it afterward.
Tests use a disposable destination, not user documents.

- Atomic snapshot replacement checks encoded initial contents, replacement run 42,
  absence of leftover siblings and rejection of oversized, invalid and missing
  files. The directory listing contains only workspace.signal.
- Failed replacement uses an existing nonempty directory as the destination,
  expects Error, confirms no temporary sibling remains and reads its original
  child contents. This checks an actual rename failure, not a mocked success.
- Exact boundary accepts a valid encoding padded with spaces to 16384 bytes.
  Saving run 51 over a symlink yields run 51 at that replaced path while the original
  target remains run 0.

let%expect_test registers each case; [%expect] checks printed values/listings.
These cases do not directly inject cancellation, collisions, permissions changes,
crash/power loss or native panels. [Desktop integration](../README.md#local-checks)
separately checks controller races and OS document metadata.

For another document format, keep bounded reads and validate before returning
an application model. For cloud storage, provide a different injected controller
capability and specify its overwrite/cancellation contract; Eio.Path rename's
local atomic replacement guarantee does not automatically apply remotely.
