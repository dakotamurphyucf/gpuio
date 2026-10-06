# How `Components` builds reusable typed views

[README](README.md) · [Source](components.ml) · [Bridge caller](main.md)
· [Bonsai caller](bonsai_component.md)

These are ordinary OCaml functions returning `Gpuio.View` trees parameterized by
the callback result type. They create no graph, model, effect scheduler, runtime,
or native window. The caller decides whether a callback returns an `Action.t`
for the reconciler or a `Bonsai.Effect.t` for the application runner.

`key` validates literal keys. `card ~title children` constructs a column with
padding, gap, rounded border, selectable text, and an explicit selection color.
Its title key is `title`. Both cards can use that key because it is scoped under
their respective parents; sibling key uniqueness, not global naming, matters.
Native text selection and copying are runtime behavior, not stored OCaml strings
changed by this component.

`counter ~value ~on_increment` returns a keyed row with an increment button and
count text. The button has explicit accessible name “Increment counter”. Its
style starts with padding, then `Style.with_state_exn` adds Focused, Hovered, and
Pressed backgrounds. Native interaction resolves these states locally rather
than invoking OCaml whenever a pointer hovers or the widget paints. The counter
function owns no count: the displayed integer is entirely supplied by the caller.

`app` adds percentage width, spacing, token-based background/foreground, a heading,
the counter, a theme button, and a two-column grid of cards. `View.grid` validates
its layout and returns `Or_error`, unwrapped for this fixed fixture. Theme tokens
resolve using the theme submitted by the caller; the function itself changes no
OS appearance setting. The Unicode text is ordinary content including an emoji
sequence, with no special callback or decoding path here.

Click the increment button in the executable: native delivery resolves
`on_increment`, returning `Action.Increment`; the worker updates its count and
calls `app` again. In the Bonsai sample, the same callback produces a state-setter
effect, so Bonsai derives an updated counter. Stable keys keep the same native
owners while text/styles change. Building a View is description construction,
not executing an action or synchronously allocating a GPUI widget.

Build and launch the owning executable from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/view_api/main.exe
_build/default/examples/view_api/main.exe
```

Use the [configured environment](../../docs/development.md) and graphical session.
There is no `components.exe`. `main.exe --self-test` checks bridge revisions/theme
updates, not the cards' physical selection or accessibility behavior. See
[typed UI contract](../../docs/design/typed-ui.md) for style inheritance, keys,
limits, and native state rules.

To adapt, pass application values and callbacks explicitly, preserve keys across
updates, and handle validation errors for user-supplied layouts. Keep I/O and
state mutation in the caller's scheduled effects rather than inside these pure
view-building functions.
