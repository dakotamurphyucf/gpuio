# Editor visit: fence one managed-row placement

[editor_visit.ml](editor_visit.ml) and its [interface](editor_visit.mli) define a
small mutable UI-domain owner for one editor placement lifetime. It records whether
that visit is active and the highest accepted native revision. It creates no Bonsai
graph, editor, native lease, I/O task or Eio scope. The retained editor controller
and managed row have separate lifetimes; this helper supplements their guards.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Open **Settings**, edit Workspace name or budget, then navigate/filter rows so the
field leaves and returns. The [settings preview](../settings_preview.ml) owns those
controls and the event flow. There is no standalone executable, asset or diagnostic
flag for Editor_visit. Toolchain setup is in [development](../../../docs/development.md).
This documentation review adds no build, IME, keyboard or platform acceptance.

Read the internal `t`, `create`, lifecycle operations, then `observe`. The public
`t` is abstract; internally it holds mutable phase Fresh/Active/Retired and an
optional `int64` revision. `create ()` starts Fresh with no revision. `activate`
allows Fresh or already Active, but raises `invalid_arg` for Retired: a new row
placement must allocate a new visit. `deactivate` marks Retired and is idempotent.
`is_active` distinguishes Active from Fresh/Retired.

`observe ~revision` returns false without modification if inactive, revision is
negative, or revision is no newer than the last accepted value. Otherwise it stores
that revision and returns true. Equal revisions are duplicates. A fresh active
visit can accept zero even if an old visit reached a larger number; revisions are
not one global counter shared by all editor mounts.

## How the caller combines visit and row lifetime

[settings_preview.ml](../settings_preview.ml) constructs the visit with
`B.Expert.thunk ~f:Visit.create` inside each managed item's graph. Its
`B.Edge.lifecycle` activates the Name/Budget visit and stores it in `name_visit` or
`number_visit` on activation. Deactivation retires the visit and clears the current
owner ref only if it still points physically to that visit; this prevents an old
placement from clearing a replacement. `Managed_rows.Lifetime.guard lifetime`
separately fences callbacks to the acknowledged row lifetime.

These are Bonsai/lifecycle operations in the caller, not in this helper. A graph
constructs retained state once; `let%arr` reads current row/item/controller values
to derive its view and callbacks. The editor remains native-owned: a callback
runs an `E.of_thunk` on the UI domain rather than changing model data while building
that view.

`mirror_name` ignores search-only events, extracts Changed or Submitted revisions
and text, then calls `Visit.observe`. It persists text only outside native composition.
`mirror_number` extracts a snapshot from all relevant number events, applies the
same revision gate, and persists committed value/raw draft only outside composition.
The revision gate is evaluated before the composition check, so an accepted composing
observation still advances the visit's revision even though it does not persist text.
Command completions also check active placement/lifetime before modifying application
state. Read [text-input controller](../../../lib/eio/text_input.mli) and
[number-input controller](../../../lib/eio/number_input.mli) for their native lease
and snapshot contracts; this helper alone cannot establish native ownership.

For a concrete trace, visit A becomes active and accepts revision 8, mirroring a
noncomposing workspace name. Revision 7 or another 8 is ignored. The row leaves;
A is retired and an application reset restores the name. A late revision 9 cannot
undo that reset. A new visit B activates and accepts its new mount's revision zero;
A's revision 99 still cannot overwrite B. The
[existing expect test](../../../test/gallery/editor_visit_test.ml) checks this exact
pattern, inactive/negative observations and invalid reactivation. No test was run
for this guide and no test source group is approved by this model review.

This owner is bounded to one phase and optional revision and must stay on the UI
domain; do not share it between independent fields or access it concurrently from
Eio producers. It needs no native release; retire it with row deactivation and let
the actual controller/window own native cleanup. To use the recipe for another
native-backed settings field, allocate a visit inside that row graph, guard its
callback by row lifetime, and call `observe` before persistence. Keep a fresh visit
per placement instead of reactivating retired state or comparing new-mount revisions
against old ones.
