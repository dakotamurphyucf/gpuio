# Application document defaults — OCH-41

Implementation contract with Core/Bonsai/driver/Eio attachment. Pure precedence,
reconciliation, application-window tests and an independent installed-gallery consumer
pass; see [evidence](../evidence/document-defaults-och41.md).

An immutable defaults value belongs to one application run and is shared by its
windows. It is not a process-global singleton. Existing applications that supply no
defaults retain current behavior. Changing a document's explicit properties remains
a normal Bonsai update; a live application-wide defaults setter is a separate API,
not implicit mutation of this value.

Each configurable reader field retains intent: `Inherit` uses the application
value, `Builtin` uses GPUIO's original fallback, and `Value x` supplies an explicit
value. Existing optional constructor parameters remain source compatible: omission
means Inherit, presence means Value, including an explicitly supplied default.
`Config.with_overrides` changes only its supplied fields and can restore inheritance
or reset an optional style/preview limit to the built-in absence.

Defaults cover appearance, layout, line-number visibility, initial collapse,
selection-copy format, preview limit, internal text style, parser options and
code/table actions. Source handles, labels, file paths, search queries, image
resources and diff expansion state remain document-specific. Markdown options apply
only to Markdown; style/actions/profiles apply only to Markdown/HTML; inherited
preview limits apply only to rich Flow documents. Explicit incompatible values
continue to fail validation. An inherited setting in an inapplicable mode uses the
built-in value; it must not make otherwise-valid Code/Diff/Viewport documents fail.

Profile and action defaults pair values with typed callbacks. A shared callback
receives the resolved document Config, including its source handle, before the
revisioned event; no default may erase the information needed to identify a source.
Per-document handlers override shared handlers. Explicit profile attachment wins;
a separate clear modifier suppresses profile inheritance. Callbacks remain queued
and source/config-generation fenced through the existing bridge.

The runtime carries defaults from App.run/run_desktop into every window driver
and reconciler, including later-created windows. It resolves current and previous
configurations consistently before wire comparison, callback binding and epoch
calculation. It preserves physical sharing of unchanged view subtrees. Failed prepare
does not publish defaults or new callbacks. No additional Rust global or protocol
opcode is needed: the bridge receives the resolved existing operations.

Acceptance requires pure precedence/mode/reset tests; Core reconciliation and
callback/source/rollback tests; application/driver wiring and multi-window isolation;
a public gallery example; installed consumer and applicable native behavior checks.
The pure layer alone does not establish application-wide behavior.


## Usage

```ocaml
let document_defaults =
  Document.Defaults.create
    ~selection_format:Markdown
    ~text_style:(Document.Style.create ~paragraph_gap_rem:1.8 () |> Or_error.ok_exn)
    ()
  |> Or_error.ok_exn
in
Gpuio_eio.App.run ~document_defaults (fun env app -> (* open application windows *) ())
```

Callbacks can be installed with `Defaults.create ~on_action` and
`Defaults.with_profile defaults instance ~on_event`. The latter keeps the typed
profile codec together with its instance; default profile callbacks receive the
resolved Config followed by the typed event. An explicit `View.with_document_profile`
overrides inheritance. `View.without_document_profile` clears it until a newly
constructed ordinary document view restores inheritance.

```ocaml
Document.Config.with_overrides config
  ~text_style:Builtin
  ~selection_format:(Value Plain_text)
  ~max_lines:Inherit
  ()
```

An omitted modifier field preserves its prior intent. Config getters before runtime
resolution show built-in fallback values for inherited fields. Resolution preserves
the original intent, so reusing an immutable config in another application does not
carry resolved defaults from the first one. Application defaults are fixed for a
run; this API does not install a mutable global callback or promise a live setter.

The gallery's `--document-defaults` option exercises the real application runner.
Its Documents comparison selects Inherit, Built-in or Compact override using the
same source. The normal gallery launch preserves previous defaults. Compilation,
expect tests and no-window catalog checks remain distinct from physical copying,
keyboard/accessibility and rendered layout qualification.
