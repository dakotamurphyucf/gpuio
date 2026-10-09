# Examples

Start with [getting_started](getting_started/README.md): one small counter with
explicit GPUIO view, Bonsai state and application startup functions. Then use the
[gallery](gallery/README.md#reading-the-code) for component recipes,
[Agent Workspace](agent_chat/README.md#implementation-map) for a larger chat app,
and [Signal Studio](signal_studio/README.md#reading-the-code) for graphics and
desktop integration. Their entry points link to modules by responsibility;
optional acceptance runners are separate from normal app code.

The gallery's [charts walkthrough](gallery/charts_page.md) traces source
publication, plotting options, native selection and page-scope cleanup.
Read its [startup](gallery/application.md), [Bonsai composition](gallery/component.md),
[layout](gallery/shell.md) and [page routing](gallery/pages.md) guides for the
application structure surrounding those components.
Its [embedded command browser](gallery/embedded_palette_preview.md) and
[external search](gallery/external_palette_preview.md) guides distinguish local
Bonsai state from native query ownership and scoped Eio producers.

The [walkthrough coverage checklist](coverage.md) inventories all example source
parts and exposes missing or unreviewed guides. See the [review guide](coverage-guide.md)
when adding or changing an example; an existing README alone does not establish
complete documentation.

The examples below include historical bootstrap and low-level integration checks.
They are useful for framework work, but are not the recommended first-app template.

`foundation/` is the bootstrap Bonsai/Eio/GPUI window and input example. Run it with
`./scripts/gpuio smoke`; `--self-test` exercises the bridge/lifecycle/input handlers
and cleanup, while `--two-windows` exercises native window identity and independent
editor lifetime. These tests open actual windows, but input probes invoke native
handlers; they do not prove real OS IME or accessibility behavior.

The bootstrap protocol and single-window application host are private to this
example. The production per-window Bonsai/Eio API is demonstrated in `runtime/`.
The integrated `agent_chat/` workspace is documented in its [README](agent_chat/README.md). Signal Studio is the current graphics reference.

`view_api/` demonstrates the public typed view/style/theme vocabulary, reusable
components, a compiled Bonsai.Cont component and an explicit Eio bridge runner.
See its [README](view_api/README.md) for interactive and automated commands.
`bridge/` separately exercises production transaction rollback and ownership.

`text_input/` demonstrates a single-line input and multiline composer, stable
Bonsai controllers, submit observations and explicit native commands. See its
[README](text_input/README.md). Local foreground GUI checks are authorized for fast iteration; avoid activation
where a test permits background execution. CI supplies the final platform gates.

- `combobox`: editable choices with native query ownership and a public controller
  self-test for guarded replacement, undo and stale unmount. See its README.

- [`presentation`](presentation/README.md): public stateless presentation helpers,
  settings/form semantics and reusable chat cards, with light/dark appearances
  and native keyboard/accessibility checks, with coverage limits in its README.

- [Positioned menus](menu_controller/README.md): open/close a context menu from
  an OCaml controller, with validated coordinates and stale-definition rejection.
  [Two-window walkthrough](menu_controller/multiwindow.md) covers independent
  controllers, activation and native editing targets.
  [Component walkthrough](menu_controller/component.md) separates state, views
  and asynchronous commands from the small window launcher.
- [Menus](menus/README.md): shared command registries, Bonsai effects, dropdowns
  and optional native OS context popups. The adjacent [walkthrough](menus/main.md)
  traces the complete interaction and explains the optional self-test.

## Browse by purpose

These entry points link to each application's adjacent implementation walkthroughs.
Start with the small examples above before reading diagnostics or extension internals.

- **Controls and editing:** [controls](controls/README.md),
  [text input](text_input/README.md), [editable combobox](combobox/README.md),
  [numeric input and OTP](numeric/README.md), [calendar/date picker](calendar/README.md),
  [color input/picker](color_input/README.md), [commands](commands/README.md),
  [command palette](palette/README.md), [menus](menus/README.md),
  [positioned menus](menu_controller/README.md).
- **Layout and presentation:** [presentation](presentation/README.md),
  [navigation](navigation/README.md), [container queries](container_query/README.md),
  [overlays](overlays/README.md), [tooltips](tooltips/README.md),
  [toasts](toasts/README.md), [progress](progress/README.md),
  [target animations](animation/README.md), [animation programs](animation_program/README.md),
  [typed view API](view_api/README.md).
- **Data and graphics:** [virtual lists](virtual_list/README.md),
  [trees](tree/README.md), [tables](table/README.md), [charts](charts/README.md),
  [documents](documents/README.md), [images](images/README.md),
  [canvas](canvas/README.md).
- **Desktop interaction:** [window lifecycle](window_lifecycle/README.md),
  [desktop services](desktop/README.md), [notifications](notification/README.md),
  [file dialogs](file_dialogs/README.md), [pointer capture](pointer/README.md),
  [internal drag/drop](drag_drop/README.md), [desktop file drag/drop](drag_drop_desktop/README.md).
- **Static extension packages:** [counter author](extension_package/README.md),
  [document-profile author](document_profile_package/README.md),
  [OCaml extension consumer](extension_consumer/README.md). Author guides identify
  the Rust work; application consumers use the packaged OCaml interface.
- **Framework diagnostics:** [historical foundation](foundation/README.md),
  [transaction bridge](bridge/README.md), [runtime/scopes](runtime/README.md),
  [asset upload](asset_upload/README.md), [canvas upload](canvas_upload/README.md),
  [chart upload](chart_upload/README.md), [chart stream](chart_stream/README.md).
  Publication, synthetic input and frame acknowledgement each prove different things.
- **Performance and resources:** [frame workload](performance/README.md),
  [large documents](performance_document/README.md), [idle](performance_idle/README.md),
  [window lifecycle](performance_lifecycle/README.md),
  [shared lifecycle workload](lifecycle_workload/README.md),
  [presentation timing](performance_presented/README.md),
  [native measurement probe](performance_probe/README.md),
  [streaming](performance_streaming/README.md),
  [paged table history](performance_table/README.md), [resource audit](resource_audit/README.md).
  Read the measurement contract before running or interpreting a benchmark.
