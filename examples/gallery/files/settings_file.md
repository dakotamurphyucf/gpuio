# Export immutable bytes through an explicit filesystem capability

[settings_file.ml](settings_file.ml) and [settings_file.mli](settings_file.mli)
implement one Eio save function; the adjacent [test](test/settings_file_test.ml)
shares this guide. Read size/path validation, parent/random sibling acquisition,
write/sync/rename and protected cleanup, then the expect test. This helper is not
Bonsai state, a native file panel, a serializer or a settings import API.

After [setup](../../../docs/development.md), from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/gallery/files -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
```

Choose Settings and Export settings. [Settings preview](../settings_preview.md)
captures validated committed values before its native panel and runs this injected
writer in a visit child of the window scope. [Application](../application.ml)
converts the selected File_path into an Eio.Path under its explicit fs capability
and supplies secure_random. This function cannot grant filesystem access from an
incoming deep link or show a panel. Commands were reviewed, not run here.

## Size, randomness and replacement guarantees

save takes an Eio.Path.t, random Flow.source and immutable string, returning
unit Or_error.t. More than 65536 bytes fails before opening anything; exactly 65536
is accepted. The limit is bytes, not Unicode character count. Path.split must
yield parent/filename. with_open_dir holds the parent open throughout. Sixteen
random bytes become 32 hex digits in a .gpuio-save- temporary sibling, created
exclusively with 0600 permissions. Collision fails rather than retries, and no
existing temporary file is truncated.

Inside with_open_out, Exn.protect writes contents, syncs the file and renames it
onto destination. Before rename the prior destination is untouched. A destination
symlink is replaced itself rather than followed. finally runs unlink under
Eio.Cancel.protect, ignoring Not_found after successful rename; other cleanup
errors are not silently swallowed. Scoped open helpers close file/directory handles.
Expected Eio.Io exceptions become Or_error; cancellation propagates. Private mode
replaces prior permissions, and file sync does not promise containing-directory
crash durability. Cancellation after rename may leave new contents installed.

There is no contents decoding here: Settings_state.encode owns format/version/
blank-name validation; callers could intentionally export other bytes under the
same limit. The helper never mutates application drafts or saved-state metadata.
An export failure therefore does not erase retained settings.

## One complete filesystem expect test

[files dune](dune) defines the Eio helper library; [test dune](test/dune) enables
ppx_expect, Eio_main and its existing theme fixture dependency. with_directory
creates a 0700 settings-export-fixture beneath the test cwd, supplies secure_random
and removes it in Exn.protect. It never targets a user settings file.

The single expect test saves initial then updated bytes and reads back updated;
rejects 65537 bytes without changing that file; attempts replacement of a nonempty
directory and checks its original child; prints a sorted listing containing only
destination/settings.sexp (no temporary siblings); replaces a symlink with
independent bytes while keeping its original target updated; finally writes and
checks the exact 65536-byte boundary. Typed assertions cover contents/errors,
while [%expect] fixes the directory listing. These are real local filesystem
operations, not mocked native Save-panel acceptance.

The test does not inject cancellation, collisions, crashes, permissions failures
or OS panel input. [Gallery native checks](../README.md) exercise the preview's
controlled writer failure, selected export and row/editor behavior separately.
Linux nongraphical test coverage is distinct from deferred desktop GUI qualification.
For adaptation, preserve the explicit capability, bounded bytes and documented
replacement contract; remote storage needs its own atomicity/cancellation policy.
