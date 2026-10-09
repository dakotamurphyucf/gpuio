# Share original SVG icons through scoped registrations

[icons.ml](icons.ml) and [icons.mli](icons.mli) hold the original monochrome SVG
fixtures, publish them once at application startup and provide view/decoration
lookups. Sources are embedded strings, so no icon file, font download or network
request is required. Bonsai observes registered handles; native GPUIO owns
encoded assets and SVG mask decoding/tinting.

Use the [isolated environment](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Observe search/send/history controls and the destination sidebar; switch themes
to see icons inherit foreground. macOS is the v1 target; Linux GUI qualification
is [informational](../../../docs/platform-release-policy.md).

## Names, initial state and publication

Read `Name`, `create`, `paths`, `initialize`, `find`, `decoration` and `view`.
`Name.t` is a closed variant with 18 names (Spark through Arrow_down), derived
typed equality for association lookup. `t` is an abstract Bonsai expert variable
holding `(Name.t * Asset.Handle.t) list`; `create ()` starts empty and `value`
exposes its reactive value. A handle is an immutable reference to one application
registration, not a file path, persistent resource ID or lifetime extension.

`paths` maps every Name to literal SVG geometry. `initialize t app` recursively
wraps each path in a 24 × 24 viewBox with no fill, white rounded strokes, then
constructs `Asset.Source.of_bytes ~format:Svg`. `let%bind` here sequences
`Bonsai.Effect` registration results; it is not graph-building `let%arr`.
`Asset.register app ~scope:(App.scope app)` publishes encoded bytes through the
[public asset controller](../../../lib/eio/asset.mli).

On success the loop collects handles and finally publishes the complete list
in original order with a deferred Var.set. Partial progress is not exposed.
Registration failure raises the typed diagnostic through the UI effect; this
helper has no retry status or partial-failure UI. Assets already registered still
belong to the application scope and retire at its end. Successful publication
means encoded bytes were accepted, not that every SVG decoded or painted.

`initialize` itself has no already-started guard. [Application.run](../application.md)
uses its resources_started flag to call it once after the first window change;
that caller enforces the interface's once-per-application use. Do not call it on
every render or theme switch: duplicate invocation would create registrations,
not recolor existing icons. The completed list is shared across all workspace
windows and released by application scope ownership.

## Look up views and control decorations

`find` uses `List.Assoc.find ~equal:Name.equal`. `decoration` returns None until a
handle exists, otherwise `Icon.Decoration.create ~asset ()`. It is intended for
control icon slots; the containing button supplies accessible name/activation.
`view` returns an empty 16 × 16 column while missing, avoiding a placeholder
label. Once present it returns a 16 × 16 nonshrinking `View.icon` with a decorative
image description. The [icon contract](../../../lib/core/icon.mli) requires SVG
and uses its alpha mask tinted by inherited native foreground; the white SVG
stroke does not force every icon to stay white.

The concrete update trace is startup registration → published handle list →
`Icons.value` dependency update → caller's `let%arr icons = icons ...` derives
icon slots → native icon decoding/rendering. The
[artifact sidebar](artifact_sidebar.md) decorates destinations with these handles,
while [message framing](chat_message.md) uses Spark directly. Neither lookup
owns registration disposal or starts Eio work. Hidden/remounted controls can
reuse the same live registration; closing one window does not release icons
needed by another. Application closure retires them all.

A small adaptation is another icon: add a Name constructor and one matching
`paths` entry, then choose it in a caller's exhaustive icon mapping. Keep SVG
geometry monochrome, the fixed viewBox consistent and meaningful accessible
labels on controls. For file/network assets, obtain bytes with explicit Eio
capabilities outside this helper's pure fixture table and preserve application
scope ownership. Existing image/theme/input checks are linked in the
[README](../README.md); this source review runs no decode or physical icon test.
