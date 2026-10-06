# Routing gallery pages and composing the first demonstrations

[`Pages.component`](pages.ml) selects the active page using Bonsai `match%sub`.
Its [interface](pages.mli) makes services and the reactive `page`/`palette` values
explicit. Read the dispatch at the end first, then `presentation`, `controls`
and `editors`: these three page bodies are implemented in this file. Other page
modules remain independent components with their own coverage entries.

Use the [application guide](application.md) to build and launch. Try
**Presentation**, **Selection & actions**, and **Text editing** in the navigation
rail. These are public OCaml demonstrations, not a mandatory framework scaffold.
The normal launch does not run native automation or establish Linux GUI coverage.

## Reactive branches and resource lifetimes

`Page.t` is a closed variant with stable keys, labels and ordering in
[`model/page.ml`](model/page.ml) and its [interface](model/page.mli). Selecting a
rail button changes that value in [Component](component.md). `match%sub` activates
the corresponding graph branch; `let%arr` inside a branch combines its current
values into a view. Branching is not the same as keeping every native page
mounted with a hidden style.

Leaving a page unmounts its native previews and deactivates lifecycle hooks.
Resource-owning examples such as charts and external search use explicit scopes
to retire registrations or producers. Some ordinary Bonsai model values can
survive an inactive branch; do not infer that all state resets from `match%sub`
alone. Native editing sessions and explicitly reset controller state have their
own contracts. [Preview_scope](preview_scope.md) explains the common acquisition
helper; [charts](charts_page.md) traces a resource-backed page in detail.

`group` is a pure column-spacing helper. It neither allocates state nor owns
children's tasks. Each preview's component is constructed before the page's
`let%arr`, so graph allocation is separate from combining current views.

## Presentation

`presentation` composes separate avatar, attachment, badge, label, loading,
message and other preview modules. It also defines a notice string and loading
animation toggle. **Try it**, **Inspect attachment** and **Create collection**
only update demonstration messages; they do not inspect a real file or create
persistent data. Loading previews start static and can be enabled explicitly.

`status_regions` is the local three-region status-bar demonstration. Its three
Boolean toggles add/remove leading, center and trailing content. A pure reducer
counts **Sync workspace** activations. Native activation delivers the effect,
the reducer increments the count, and a derived status text updates. The
`Presentation.status_bar` helper handles the layout, while the surrounding
named accessibility group supplies context. Neither operation performs I/O.

## Selection and actions

`controls` defines three validated `Choice.Id` values and a shared immutable
collection. Selected state starts at `balanced`. Radio group, dropdown and
editable combobox all use that selected ID; **Enable selection controls** drives
their disabled policy. A checkbox and button counter demonstrate independent
state beside the specialized preview components.

For a dropdown selection, its native callback delivers a `Choice.Id`, Bonsai
stores it, and the three configurations derive the same new selection. The
combobox's native query remains owned by `Gpuio_eio.Combobox`, distinct from the
application's selected ID. Its controller is created once in the graph from a
reactive configuration and callback, then rendered with `Combobox.view`.

## Text editing

`editors` has a second closed variant, `Editor_section`, for **Forms & basic
editing**, **Native input options**, and **Multiline & search**. Each branch owns
its relevant previews. The UI explicitly says switching groups starts fresh
editing sessions; it does not promise draft persistence between editor groups.

`basic_editors` creates a native single-line title and multiline body. The title
starts as `A place for good ideas`; the body includes CJK text and a joined emoji
sequence. Read-only and clear-on-Escape state derive both configurations. Title
submission copies the committed text into a Bonsai `submitted` value; the
multiline body keeps its own editing session. Native text/selection/undo are not
mirrored character-by-character into the page model.

**Show validation error** changes the form-field presentation, independently of
read-only/editing policy. It is a demo toggle, not a validation algorithm. The
form wrapper supplies a label, help and optional error around the retained title
control. In Options, password, content-hint, format-mask and edit-filter previews
are separate modules; Multiline delegates to `Textarea_preview`.

## Adding a page

Add a constructor and its `all`, `key`, `title` and `description` cases to
`Page`, then add the exhaustive dispatch here. Keep the key stable if the title
changes. Put an independently meaningful page in its own module and adjacent
walkthrough, update the [coverage inventory](../coverage-guide.md), and pass only
the capabilities it uses. A new page that needs I/O should use a scoped producer;
placing a task in a row renderer or `let%arr` gives it the wrong lifetime.

To add a fourth selection mode instead, extend the `choices` data while preserving
unique IDs. No new native adapter or Rust code is required. Verify disabled and
selection behavior rather than assuming a label change proves all consumers
remain synchronized.
