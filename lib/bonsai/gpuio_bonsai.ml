module Managed_rows = Managed_rows
module Virtual_list = Virtual_list
module Tree_rows = Tree_rows
module Tree = Tree
module Table = Table
module Command_binding = Binding_observer
module Settings = Settings_panel

module View = struct
  type t = unit Bonsai.Effect.t Gpuio.View.t
  type toast = unit Bonsai.Effect.t Gpuio.View.toast

  let with_key = Gpuio.View.with_key
  let with_accessibility = Gpuio.View.with_accessibility
  let drag_source = Gpuio.View.drag_source
  let drop_target = Gpuio.View.drop_target
  let command_binding_scope = Gpuio.View.command_binding_scope
  let highlight_scope = Gpuio.View.highlight_scope
  let input_region = Gpuio.View.input_region
  let pointer_area = Gpuio.View.pointer_area
  let toast = Gpuio.View.toast
  let toast_stack = Gpuio.View.toast_stack
  let icon = Gpuio.View.icon
  let animate = Gpuio.View.animate
  let container_query = Gpuio.View.container_query
  let animate_program = Gpuio.View.animate_program
  let canvas = Gpuio.View.canvas
  let chart = Gpuio.View.chart
  let document = Gpuio.View.document
  let image = Gpuio.View.image
  let text = Gpuio.View.text
  let styled_text = Gpuio.View.styled_text
  let with_text_shimmer = Gpuio.View.with_text_shimmer

  let link ?key ?style config ~on_click children =
    Gpuio.View.link ?key ?style config ~on_click:(fun () -> on_click) children
  ;;

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
  let sheet = Gpuio.View.sheet
  let alert_dialog = Gpuio.View.alert_dialog
  let popover = Gpuio.View.popover
  let command_scope = Gpuio.View.command_scope
  let slider = Gpuio.View.slider
  let rating = Gpuio.View.rating
  let avatar = Gpuio.View.avatar
  let loading = Gpuio.View.loading
  let progress = Gpuio.View.progress
  let command_palette = Gpuio.View.command_palette
  let menu_button = Gpuio.View.menu_button
  let context_menu = Gpuio.View.context_menu
  let menu_bar = Gpuio.View.menu_bar
  let command_button = Gpuio.View.command_button
  let tooltip = Gpuio.View.tooltip
  let hover_card = Gpuio.View.hover_card
  let extension = Gpuio.View.extension
  let split_pane = Gpuio.View.split_pane
  let tab_bar = Gpuio.View.tab_bar
  let tab_panel = Gpuio.View.tab_panel
  let panel = Gpuio.View.panel
  let carousel = Gpuio.View.carousel
  let navigation_stack = Gpuio.View.navigation_stack
  let accordion = Gpuio.View.accordion
  let disclosure_with_header = Gpuio.View.disclosure_with_header

  let disclosure
        ?key
        ?style
        ?trigger_style
        ?panel_style
        ~label
        ~expanded
        ?disabled
        ~hidden
        ~on_toggle
        children
    =
    Gpuio.View.disclosure
      ?key
      ?style
      ?trigger_style
      ?panel_style
      ~label
      ~expanded
      ?disabled
      ~hidden
      ~on_toggle:(fun () -> on_toggle)
      children
  ;;

  let radio_group = Gpuio.View.radio_group
  let select = Gpuio.View.select
  let combobox = Gpuio.View.combobox
  let virtual_list = Gpuio.View.virtual_list
  let column = Gpuio.View.column
  let grid = Gpuio.View.grid
end
