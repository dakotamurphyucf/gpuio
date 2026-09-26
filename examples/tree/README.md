# Managed filesystem tree

Run from the repository root:

```sh
./scripts/gpuio exec dune exec examples/tree/main.exe
```

The example uses `Gpuio_bonsai.Tree.component`, component-owned selection and
expansion, and `Gpuio_eio.Tree_loading` in an application scope. File operations
use the explicit Eio current-directory capability in loading workers. Rendering
performs no filesystem I/O. Native rows stay within a 32-row active budget;
selection preferences do not pin offscreen rows.

Use arrows, Home/End, Space/Enter, platform selection modifiers and Unicode
typeahead. Disclosure hit regions add no extra Tab stop; native TreeItem keyboard
and accessibility actions own expansion. Loading and retry rows are separate
from application items. Mode switches preserve eligible selection, and Reload
creates a new source generation that retires old commands and row models.

Capture a payload-free target before creating a command:

```ocaml
let target = Tree.Output.target output file_id |> Or_error.ok_exn in
let reveal =
  Tree.Controller.reveal (Tree.Output.controller output) ~focus:true target
in
(* Schedule [reveal] as a Bonsai effect. *)
```

The high-level widget opens loaded ancestors and waits for the updated projection
before requesting native reveal/focus. An absent ID is an error when capturing a
target. A stale target is ignored at delivery; reusing a filename does not redirect
an old command to its replacement. Observing `Tree.Output.state` lets an app save
preferences outside the component and supply them as seeds on later mounts.

Filesystem pages contain at most 128 children. Symlinks are leaves, and directory
reads are not recursive. The example uses validated relative path IDs, so long or
non-UTF-8 names can produce a retryable page error under the framework's ID/label
budgets. Eio's directory-list allocation and the example's application payloads
are outside native row budgets. A directory over 100,000 entries is rejected.
This example does not watch files or promise stable paging across filesystem
mutations: changed listings can invalidate a page, and Reload starts fresh.

The local native self-test uses this checkout's `test/virtual_list` directory:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune exec examples/tree/main.exe -- --self-test
```

It loads real directories, reveals a file, waits for its native focus retention
pin, checks selection and stale-command retirement after reset, then closes the
application. This is separate from physical keyboard/assistive-device tests and
from the full large/deep native tree workload required by OCH-38.
