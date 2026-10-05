# Ordinary text-area layout

OCH-41 adds `Text_area_layout` to `Text_input.Config.create ?layout` for
`Multiline` fields. Single-line configurations reject it, including an explicit
default value. Rust retains the same native text area; changing layout never
reseeds its text, changes its editor revision or clears composition/history.

```ocaml
let layout =
  Text_area_layout.create
    ~soft_wrap:true
    ~wrapping_indent:Match_first_line
    ~show_whitespace:false
    ~cursor_margin_lines:3
    ()
  |> Or_error.ok_exn
in
Text_input.Config.create ~mode:Multiline ~label:"Notes" ~layout ()
```

Defaults are soft wrapping, continuation indentation matching the first line,
hidden whitespace indicators and the native automatic cursor margin. Indentation
can instead be `Flush_left`. It changes continuation layout, never the underlying
whitespace. Turning wrapping off permits horizontal scrolling; turning it on
resets horizontal scroll to zero. Reflow and viewport changes can clamp offsets.
Unrelated label/read-only updates do not reapply wrapping or reset horizontal
scroll. Native whitespace indicators are a presentation setting, not text edits.

An explicit cursor margin is 0..256 rendered line heights, saturated to half the
viewport capacity. It controls caret-follow scrolling during directional
navigation. Explicit selection commands and pointer placement use native minimal
reveal, rather than always recentering the field. Omitting the override preserves
the engine's automatic behavior. A margin cannot create document space beyond
the first/last line. Fixed and auto-growing fields share this contract; both use
the pinned engine's auto-grow mode, with equal min/max rows for fixed sizing.

## Bridge and ownership

The unpublished epoch-3 protocol appends Op78 `Set_text_area_layout` /
`SetTextAreaLayout` with optional configuration. `None` restores defaults.
Wrapping indent tags are 0 flush-left and 1 matching the first line. The optional
margin uses signed bin_prot integers; both decode and native admission reject
values outside 0..256. Legacy `EditorConfig` bytes are unchanged.

The reconciler emits only changed layout options. The native tree accepts this
operation only for Textarea nodes, accounts a fixed configuration payload and
rolls back failed transactions, including revision and resource charges. Native
configuration compares individual fields before calling setters, preventing
unrelated updates from triggering reflow or resetting scroll.

The existing GPUI Base adaptation honors explicit cursor margins in auto-grow
mode and in directional scrolling outside code-editor mode. Automatic defaults
remain unchanged. The canonical patch/hash and reconstruction script carry this
change; builds do not depend on scratch artifacts. See
[validation evidence](../evidence/textarea-layout-och41.md).

## Scope and remaining acceptance

The public gallery's Editors page includes “Room to write,” with wrapping,
indentation, whitespace and cursor-margin controls on a retained draft. Native
TestPlatform geometry/composition tests are separate from macOS physical
keyboard/IME and visual acceptance. Whitespace glyph appearance still needs the
physical gallery walkthrough; setter wiring alone is not visual evidence.

Search/replace presentation and asynchronous viewport observations/commands are
separate remaining ordinary-text-area work. Code-editor diagnostics, folding,
multicursor editing and LSP remain post-v1. The pinned engine's
`scroll_beyond_last_line` is code-editor-only at paint time; it is deliberately
not exposed as an ineffective ordinary-text-area property.

Real Japanese IME composition/candidate/commit/undo/cancel now has
[wrapped and nonwrapping macOS evidence](../evidence/multiline-ime-cancellation-och17.md),
including vertical and horizontal reveal. This does not replace the remaining
whitespace/indentation/cursor-margin visual walkthrough.
