# Composed-link walkthrough

Read [link_preview.ml](link_preview.ml) and its [interface](link_preview.mli). [pages.ml](pages.ml) mounts `component app window palette graph` on **Presentation**.

[Preview_scope](preview_scope.md) registers the inline arrow SVG through `Gpuio_eio.Asset.register`; `Effect.map` extracts a borrowed handle or registration error. Loading/Failed branches render status before Ready builds links. Registration remains visit-owned even if icons are hidden. Departure cancels pending acquisition/disposes resources; returning acquires a fresh registration. Native passive descendants borrow this asset rather than owning the registration.

Reactive toggles own disabled, Tab inclusion, skipping Release, reversed traversal, descriptions, icons and rich previews. Tab inclusion/descriptions/icons start true. `B.state_machine0` returns current `(count, last_name)` and `click`, whose injected string action increments the latest count and records its name when executed. `let%arr` derives views from current values. Creating `click name` does not open or count a destination immediately.

The local `link` function builds [`Link.Config`](../../lib/core/link.mli) with meaningful label “Open <name>”, disabled state, explicit Tab stop and ordering index. Source order is Design/API/Release, but indices 10/30/20 give normal Tab order Design/Release/API; reverse changes indices to `40 - order`, giving API/Release/Design. It does not reorder the source list. Skip Release affects its Tab stop only; disabling prevents activation/focus. Non-Tab links remain pointer-focusable.

[`Presentation.composed_link`](../../lib/core/presentation.mli) wraps passive content into one accessible action, keyed by destination name. A stable inner `content` row holds an optional icon and text column. Rich mode substitutes a decorative asset-backed avatar, image or animated spinner for the three destinations; otherwise all show the decorative arrow. Detailed mode adds subtitle text. The checked passive-content contract permits media/loading but rejects interactive descendants; place another action outside the link rather than nesting it.

Native activation of Design guide runs the supplied callback and its `click "Design guide"` effect, updates the activity model, and causes `let%arr` to derive new count/last-opened text for native reconciliation. Nothing opens a URL: these are mock gallery actions. Enable rich previews to see media composition; the Release spinner is animated artwork, not this link’s loading flag and does not suppress activation. Change Tab options and compare traversal with the declared indices.

GPUIO owns focus, activation, media rendering and spinner animation; Bonsai owns options/activity, and the scope owns the SVG registration. There is no network request or per-frame OCaml loop. Adapt by supplying explicit routing/open effects, meaningful accessible names and stable destination keys. If real work makes a link busy, use Link.Config loading policy and compose its visible status independently rather than treating a spinner as an activation guard.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks executed for this documentation change. This component has no standalone executable or self-test. Native interaction requires the running gallery; compilation does not establish focus, keyboard, accessibility or platform acceptance. See [gallery instructions](README.md) and the [development environment](../../docs/development.md).
