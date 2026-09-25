module Managed_rows = Managed_rows
module Virtual_list = Virtual_list

module View = struct
  type t = unit Bonsai.Effect.t Gpuio.View.t
  type toast = unit Bonsai.Effect.t Gpuio.View.toast

  let drag_source = Gpuio.View.drag_source
  let drop_target = Gpuio.View.drop_target
  let pointer_area = Gpuio.View.pointer_area
  let toast = Gpuio.View.toast
  let toast_stack = Gpuio.View.toast_stack
  let icon = Gpuio.View.icon
  let animate = Gpuio.View.animate
  let document = Gpuio.View.document
  let image = Gpuio.View.image
  let text = Gpuio.View.text

  let button
        ?key
        ?style
        ?accessible_name
        ?disabled
        ?leading_icon
        ?trailing_icon
        ~on_click
        text
    =
    Gpuio.View.button
      ?key
      ?style
      ?accessible_name
      ?disabled
      ?leading_icon
      ?trailing_icon
      ~on_click:(fun () -> on_click)
      text
  ;;

  let icon_button ?key ?style ?disabled ~label ~on_click icon =
    Gpuio.View.icon_button
      ?key
      ?style
      ?disabled
      ~label
      ~on_click:(fun () -> on_click)
      icon
  ;;

  let checkbox ?key ?style ?accessible_name ?disabled ~state ~on_toggle text =
    Gpuio.View.checkbox
      ?key
      ?style
      ?accessible_name
      ?disabled
      ~state
      ~on_toggle:(fun () -> on_toggle)
      text
  ;;

  let switch ?key ?style ?accessible_name ?disabled ~checked ~on_toggle text =
    Gpuio.View.switch
      ?key
      ?style
      ?accessible_name
      ?disabled
      ~checked
      ~on_toggle:(fun () -> on_toggle)
      text
  ;;

  let row = Gpuio.View.row
  let focus_scope = Gpuio.View.focus_scope
  let dialog = Gpuio.View.dialog
  let popover = Gpuio.View.popover
  let command_scope = Gpuio.View.command_scope
  let progress = Gpuio.View.progress
  let command_palette = Gpuio.View.command_palette
  let menu_button = Gpuio.View.menu_button
  let context_menu = Gpuio.View.context_menu
  let menu_bar = Gpuio.View.menu_bar
  let command_button = Gpuio.View.command_button
  let tooltip = Gpuio.View.tooltip
  let split_pane = Gpuio.View.split_pane
  let tab_bar = Gpuio.View.tab_bar
  let tab_panel = Gpuio.View.tab_panel
  let radio_group = Gpuio.View.radio_group
  let select = Gpuio.View.select
  let combobox = Gpuio.View.combobox
  let virtual_list = Gpuio.View.virtual_list
  let column = Gpuio.View.column
  let grid = Gpuio.View.grid
end
