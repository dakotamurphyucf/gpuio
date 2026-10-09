# Acquiring resources for an active gallery component

[`Preview_scope.acquire`](preview_scope.ml) is a reusable lifetime helper. Read
its [interface](preview_scope.mli) before the implementation. It has no window or
visual presentation of its own; the [external palette](external_palette_preview.md)
is one runnable consumer. Use that guide's gallery build/run commands.

The result is a reactive `Loading | Ready of 'a | Failed of Error.t`. The caller
provides a window, diagnostic name and `create` effect. The effect must acquire
every resource through the supplied child scope. It returns a value or error;
there is no unrestricted global resource registry in this helper.

`B.state` stores the result. `B.Edge.lifecycle` runs effects when this component's
Bonsai branch activates or deactivates. Activation cancels any previous scope,
creates a fresh child of the window scope and invokes `create`. A successful
result changes the reactive value to `Ready`, allowing the caller's view to mount
the corresponding widget. A creation failure cancels partial acquisitions and
sets `Failed`. Failure to create the scope also becomes `Failed`.

The private `current` ref remembers only the owned scope; it is allocated once
when the graph is constructed. If an asynchronous `create` finishes after the
scope has been cancelled, `Scope.is_active` suppresses its result. Deactivation
cancels producers/registrations and resets the displayed state to `Loading`.
Returning to that branch acquires a new scope instead of reusing retired handles.
The owning window's closure also bounds the child lifetime.

For the external palette, `create` returns the scope itself; a later query starts
tasks there. Other consumers can return a scoped image/chart/controller resource.
This helper does not draw a spinner or error panel, automatically retry a failed
acquisition, or keep work alive when a page is left. The caller decides what each
state looks like. Application-wide work that must survive page changes belongs
in an application-owned scope instead.

To adapt it, write a `create` function using the supplied scope and handle all
three result constructors in the caller. Do not allocate a native handle before
activation or stash it after cancellation. This is ordinary OCaml/Bonsai/Eio
composition; native Rust ownership stays behind the public resource APIs. See
[`Scope`](../../lib/eio/scope.mli) and the
[runtime contract](../../docs/design/runtime.md) for the wider ownership model.
