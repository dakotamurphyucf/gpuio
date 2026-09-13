module View = struct
  type t = unit Bonsai.Effect.t Gpuio.View.t
  type toast = unit Bonsai.Effect.t Gpuio.View.toast

  let toast = Gpuio.View.toast
  let toast_stack = Gpuio.View.toast_stack
  let text = Gpuio.View.text

  let button ?key ?style ?accessible_name ?disabled ~on_click text =
    Gpuio.View.button
      ?key
      ?style
      ?accessible_name
      ?disabled
      ~on_click:(fun () -> on_click)
      text
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
  let radio_group = Gpuio.View.radio_group
  let select = Gpuio.View.select
  let combobox = Gpuio.View.combobox
  let column = Gpuio.View.column
  let grid = Gpuio.View.grid
end
