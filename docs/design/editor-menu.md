# Native edit menu bound to an editor

OCH-41, 2026-10-01. Local implementation under validation. This is the
Cut/Copy/Paste/Select-all portion of the [plain-input extension plan](plain-input-extensions.md),
not complete input-family or desktop acceptance.

## Public interface

`Editor_menu.t` configures an enabled flag and five validated labels: the menu
name, Cut, Copy, Paste and Select all. Defaults are enabled with English labels;
applications can supply translated nonblank UTF-8 labels of up to 4,096 bytes
each. The fixed action set matches the pinned ordinary input menu. Existing
native Undo/Redo shortcuts and registry actions remain available separately.

Use `Gpuio.View.editor_menu` or `Gpuio_bonsai.View.editor_menu` around a direct
single-line or multiline `text_input` view. For an Eio controller:

```ocaml
Gpuio_eio.Text_input.view editor
|> Gpuio_bonsai.View.editor_menu
     ~config:(Gpuio.Editor_menu.create ~enabled:menu_enabled () |> Or_error.ok_exn)
|> Or_error.ok_exn
```

The helper declares a private command scope and a context-menu wrapper. It accepts
optional style, menu appearance and key; the key defaults to the input's key.
Labels, enabled state and appearance can change without replacing the editor.
Keep the wrapper mounted: conditionally adding/removing an ancestor changes native
placement. Other child kinds, including Combobox, return a typed construction
error. Rich prefix/suffix/reveal/clear/loading adornments remain separate work.

## Native behavior and ownership

Right-click or Shift-F10 opens the menu. Right-click follows the input engine's
caret/selection placement; keyboard opening preserves selection. Escape and
successful selection return focus to the owning field. Menu navigation and actions
execute in Rust without an OCaml editing callback or text round trip. No new
keyboard command registrations are installed by the helper.

Opening captures the exact native node generation and a weak focus handle. Every
rendered action targets that field, even if another editor was previously focused.
It never falls back to the window's last editing target. Replacing or retiring the
field, losing eligibility, or changing the menu definition closes the menu. Menu
state uses the existing bounded renderer/cache and retains no extra editor entity.

Availability is native and current: disabled/hidden/modal-blocked/composing fields
cannot open it; active composition also prevents actions. Copy requires a
nonempty copyable selection; hidden password policy disables it. Cut additionally
requires editability. Paste requires editability and currently readable nonempty
clipboard text. Select all requires nonempty text. Existing input limits and
single-line normalization still govern pasted contents. Clipboard readiness does
not claim complete Wayland clipboard qualification.

GPUI defers action dispatch. Bound menu delivery therefore checks the current node,
native focus identity, policy, registry generation, menu definition/source and
focus again after deferral. It then dispatches synchronously to that exact focus
node after releasing the host View borrow. A password that is hidden at delivery, or a field replaced
after the click, cannot receive the earlier action. Focus moving to
another field cancels delivery instead of redirecting it. Generic application
command menus keep their existing window-editing-target behavior.

## Paired protocol and acceptance

`MenuPresentation.EditorContext` appends presentation tag 4 in the paired,
unpublished epoch-3 protocol. It uses existing Op18 `Set_menu`/`SetMenu`; previous
menu representations are unchanged. Admission requires exactly one direct Input
or Textarea child and native command references only. Registry changes that would
turn an edit action into an OCaml callback are rejected atomically. Public helpers
create four scoped native commands and no shortcuts.

Independent OCaml/Rust bytes, malformed input, atomic admission and Core identity
tests cover the wire/domain boundary. TestPlatform exercises the production View,
menu, input engine, clipboard, focus and deferred action delivery. This does not
prove actual macOS menu pixels, physical input, OS clipboard ownership, AX/VoiceOver,
installed-consumer behavior or Linux desktop acceptance. Those remain release
checks. The gallery password card uses the public helper and lets the user change
menu availability while retaining the field. Broader verification results are
recorded in [the evidence checkpoint](../evidence/editor-menu-och41.md).
