# Menu section labels

`Menu.Item.Label "Editing"` adds a noninteractive text row to a drawn menu.
It is presentation metadata, has no command ID, and never resolves or invokes a
registry action. Its text must be nonblank UTF-8 without NUL, at most 4,096 bytes.
It counts toward the existing 1,024-item and 256-KiB text limits shared across the
menu collection. The eight-level nesting limit remains in force. Native decoding
bounds string allocation before admitting a menu, including aggregate text.

Labels work in button dropdowns, context menus, editor context menus, nested
submenus and `View.menu_bar ~platform:false`. The default platform menu bar
rejects labels, including nested labels, through `Or_error`; the native protocol
also rejects them. This preserves platform semantics without fabricating disabled
commands. Applications wanting section labels can select a drawn menu bar.
OS-native popup menus are separate pending catalog work.

Native keyboard navigation, Home/End and typeahead skip labels. Opening a menu
selects its first enabled command or submenu; a label-only menu has no selectable
row and remains dismissible. Labels have no focus or press action, are exposed as
ordinary accessibility text, and are not marked as disabled controls. Existing
menu focus restoration, command eligibility, generation fencing and asynchronous
invocation semantics remain unchanged. Label rows use the menu's existing row
appearance and sizing; a dedicated heading style is not implied.

The wire appends `Label` after the three existing menu item constructors (tag 3).
Existing command/separator/submenu encodings remain unchanged. Both language APIs
and the native backend are released together under the existing protocol version
policy; there is no promise that an older backend accepts the new constructor.

Per-item icons and custom rich row content remain separate catalog work. This
addition does not establish VoiceOver qualification or Linux desktop acceptance.
