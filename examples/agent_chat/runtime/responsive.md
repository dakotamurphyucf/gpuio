# Select small presentation alternatives by native width

[responsive.ml](responsive.ml) and [responsive.mli](responsive.mli) expose
`at_width ~key ~height ~breakpoint ~compact ~wide`. It constructs one fixed-height
native container query around two already-created views. It does not inspect a
window size in OCaml, install a resize callback, own Bonsai state or start a task.
Keep editors, documents and producer owners outside these small alternatives.

From the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Resize the sidebar/inspector split dividers and inspect branding, conversation
commands and inspector heading/navigation. These choices follow each assigned
container width rather than a single application-wide desktop width. macOS is
the v1 target; Linux graphical qualification is
[informational](../../../docs/platform-release-policy.md).

## Construct the rule and stable branch identity

`at_width` validates branch IDs compact/wide and constructs a
`Container_query.Range` with `minimum:breakpoint`. Minimum is inclusive; compact
is the fallback below it, and wide is selected at/above it. No maximum or height
predicate is supplied. `Predicate.create ~width ()`, `Rule.create` and
`Config.create ~default:compact_id` form the declarative rule. Read the
[container query interface](../../../lib/core/container_query.mli) for finite,
nonnegative bounds and first-matching-rule semantics.

`View.container_query` receives a validated application key, 100% width,
explicit logical-pixel height, Min_width 0 and Shrink 0. This explicit geometry
matters: a branch's intrinsic size must not determine the container being tested,
which could feed its own presentation choice back into layout. `compact` and
`wide` use stable branch IDs, not list positions. The public view validates that
all referenced branch presentations are supplied exactly once.

The helper uses `Or_error.ok_exn`/validated literal style lengths; invalid caller
keys, heights or breakpoints raise rather than silently choosing a default.
If values come from users/configuration, validate them before calling this
literal-oriented helper or expose a result-returning wrapper.

## Concrete callers and a resize trace

[workspace.ml](workspace.ml) uses sidebar-brand with height 48/breakpoint 200,
and conversation-toolbar with height 45/breakpoint 720.
[inspector.ml](inspector.ml) uses inspector-heading with height 45/breakpoint 420
and inspector-navigation with height 36/breakpoint 400. Those callers build each
branch from the same current model/effects. Compact controls preserve meaningful
labels/navigation while reducing visible text or choices; the helper itself
does not define those policies.

When a native divider changes pane width, native layout assigns the query its
new width and chooses the corresponding branch. It paints/admits native input
only for the selected presentation. Both alternatives remain constructed in
Bonsai; resizing does not execute `match%sub`, cancel a task or rebuild an editor
model merely to choose a branch. Ordinary model/theme changes can still derive
new branch views through the caller's `let%arr` expressions.

The [view contract](../../../lib/core/view.mli) keeps named branch presentations
retained; hidden branches do not paint or receive native input. No `on_select`
observation is installed here. The public optional observation would be
paint-confirmed and asynchronous, not a synchronous layout input; this example
correctly lets native rules drive selection without another OCaml update loop.
The pane minimum sizes and surrounding split layout remain constraints;
container queries do not guarantee every three-column arrangement fits a tiny
window. Closing the inspector is an explicit user control for smaller layouts.

For a small adaptation, change the sidebar branding breakpoint after checking
both branch widths/labels. Keep stable query/branch keys and matching fixed
height, and keep substantial stateful controls outside duplicated alternatives.
If creating another query under the same parent, supply a distinct sibling key.
A window's closure retires native query views with its normal ownership; this
helper retains no independent handles or data caches. It uses no assets itself.

`python3 scripts/test_agent_chat_responsive.py` is the optional macOS pointer/
keyboard/focus/layout runner. The [README](../README.md) and
[existing evidence](../../../docs/evidence/agent-chat-m5.md) describe actual width/
motion/independent-window checks. This source/link review supplies no new resize,
keyboard or hidden-branch accessibility acceptance; an accepted view description
alone does not establish physical presentation.
