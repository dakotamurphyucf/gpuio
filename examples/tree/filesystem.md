# How `Filesystem` loads one directory page

[README](README.md) · [Implementation](filesystem.ml) · [Interface](filesystem.mli)
· [Caller](filesystem_demo.md)

This module is an Eio loader and pure tree constructor, not a Bonsai component.
It performs no rendering and opens no window. `Filesystem_demo.run` captures
`Eio.Stdenv.cwd env` and passes that path capability to `load`; the module does
not obtain a global filesystem handle or start its own worker.

`Entry.t` records relative path and `Eio.File.Stat.kind`. `root_id` is validated
`root`; `id relative` validates `path:` plus the relative path. `initial ()`
constructs one Workspace directory with `Branch { ids = []; next = More None }`.
An empty unloaded branch differs from a leaf: expansion can request its children.
`Tree.create` validates the complete forest. See [tree values](../../lib/core/tree.mli).

`relative` translates the root to an empty path or checks a `path:` identity.
It rejects empty, `.` and `..` segments, preventing ordinary parent traversal in
these request IDs. `load` reads `Request.parent` and parses the cursor as an
integer offset, accepting only 0 through `Tree.max_nodes`. The cursor is example
paging state, not a child identity.

Opening `Or_error.Let_syntax` enables `let%bind` and `let%map` here. These sequence
validated results; they are **not** Bonsai reactive bindings or Eio effect binds.
A failed ID, node, or cursor returns an error without publishing a partial page.
`Or_error.all` combines the per-entry validations.

`Eio.Path.read_dir (root / parent)` reads the directory listing through the supplied
capability. The module rejects listings above the node budget and takes at most
128 entries from the current offset. For each name it constructs a relative path,
validates its identity, and calls `Eio.Path.stat ~follow:false`. Directories become
unloaded branches; files, symlinks, sockets, and other kinds become leaves.
Symlinks are not recursively expanded. Node labels use the entry names, while
payloads retain path/kind for application use.

The returned `Tree_loading.Page` has page roots, their node associations, and
`End` or `More (Some next_offset)`. The scoped loader admits that page, publishes
a reactive snapshot, and the tree widget derives visible rows. No filesystem
operation occurs in Bonsai view derivation or native layout. Only `Eio.Io` errors
are converted to retryable `Or_error` failures; cancellation propagates to its
worker owner instead of being disguised as a page failure.

Directory contents can change between offset pages. Long or non-UTF-8 paths and
labels can exceed framework validation, and listing/stat races can fail. The
128-entry page bounds returned children, not the allocation for the full directory
listing. This is no file watcher or stable transactional snapshot; Reload creates
a fresh source. Adapting it to remote or mutable listings needs stable cursors and
explicit consistency policy. Path-string validation also should not be presented
as a complete filesystem security policy under concurrent mutation.

Build and exercise the owning executable from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
_build/default/examples/tree/main.exe
_build/default/examples/tree/main.exe --self-test
```

Use the [repository toolchain](../../docs/development.md) and native graphical
session. The self-test reads known checkout paths and checks loading, native
focus/selection, and reset; it is not an isolated unit test of arbitrary directory
races or names. For another root, pass an explicit Eio directory capability and
keep worker/scope ownership in the caller.
