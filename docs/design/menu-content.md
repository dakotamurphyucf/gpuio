# Rich content in drawn menus

`Menu.Item_path.of_list` validates a zero-based root menu index followed by one
or more item indices. `[0; 4; 1]` identifies the second item in the fifth item's
submenu of the first menu. Paths identify positions, so repeated command IDs
and non-command submenu/section labels can have independent presentation.

`View.with_menu_item_content menu_view ~items` replaces labels at those paths
with passive view trees: icons, images, text, containers, avatars, loading and
callback-free animations. Native rows retain the command/submenu/section label's
accessible name, checked state, enabled policy, action, submenu indicator and
keyboard navigation. Decorative descendants do not become separate accessible
controls. Icons use ordinary registered SVG resources, with caller-owned leases;
there is no synchronous OCaml callback during layout or paint.

A composed label uses the same public view vocabulary as the rest of the app:

```ocaml
let menu = Menu.create ~label:"File" [ Command save_id ] |> Or_error.ok_exn in
let path = Menu.Item_path.of_list [ 0; 0 ] |> Or_error.ok_exn in
let content = View.row [ View.icon icon_config; View.text "Save document" ] in
View.with_menu_item_content
  (View.menu_button ~menu ())
  ~items:[ path, content ]
```

The surrounding command registry supplies `save_id` and its accessible label;
`icon_config` refers to an SVG registered in the application's Eio scope. The
Bonsai-specialized `Gpuio_bonsai.View` exposes the same helper.

The helper accepts direct menu buttons, context menus and drawn menu bars.
`View.editor_menu ~item_content` supplies the same feature for the retained
native editor convenience wrapper. Platform menu bars reject custom content.
Unknown/duplicate/separator paths and interactive content are errors. Omitted
paths retain their string labels, and an empty list removes custom content.
The total forest, including internal slots, is bounded to 4,096 nodes and 128
levels; existing collection limits remain. Menus use their configured uniform
row height; custom content must fit that height. Variable per-item height is not
introduced by this contract.

Content identity follows its positional path; moving content to a different
path remounts it. Internal structural keys cannot collide with application keys.
Changing content at a surviving path leaves the menu owner and context editor
in place. Menu-definition changes retain the existing close/reset policy.

The wire uses the existing view tree. A decorated menu has one internal container
slot per declared item in preorder, including empty slots, with zero or one
passive content child each. A context menu keeps its target at child zero before
the slots; separator slots must be empty. Native admission validates this exact
shape and descendant changes atomically. Old undecorated child shapes remain
valid. Existing byte encodings are unchanged; older backends reject the new tree
shape under the repository's coordinated library/backend release policy.

The native renderer constructs only requested row content and checks the current
menu definition, child identities and open submenu path before resolving a slot.
It uses the existing passive control-label renderer for resource ownership,
decorative semantics and animation paint scheduling. No separate image registry,
Rust callback object or serialized arbitrary UI language is introduced.

This contract does not implement OS-native popup menus, richer palette policies,
or full catalog acceptance. Validation evidence is tracked separately in OCH-41.
