module Managed_rows = Managed_rows
module Virtual_list = Virtual_list
module Selectable_list = Selectable_list
module Tree_rows = Tree_rows
module Tree = Tree
module Table = Table
module Table_view = Gpuio.Table_view
module Command_binding = Binding_observer
module Settings = Settings_panel

module View = struct
  type t = unit Bonsai.Effect.t Gpuio.View.t
  type toast = unit Bonsai.Effect.t Gpuio.View.toast

  let with_hover = Gpuio.View.with_hover
  let with_key = Gpuio.View.with_key
  let with_scrollbar = Gpuio.View.with_scrollbar
  let with_window_region = Gpuio.View.with_window_region
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
  let without_document_profile = Gpuio.View.without_document_profile
  let with_document_profile = Gpuio.View.with_document_profile
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
        ?config
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
      ?config
      ?accessible_name
      ?disabled
      ?leading_icon
      ?trailing_icon
      ~on_click:(fun () -> on_click)
      text
  ;;

  let icon_button ?key ?style ?config ?disabled ~label ~on_click icon =
    Gpuio.View.icon_button
      ?key
      ?style
      ?config
      ?disabled
      ~label
      ~on_click:(fun () -> on_click)
      icon
  ;;

  let button_with_content ?key ?style ?config ?disabled ~accessible_name ~on_click content
    =
    Gpuio.View.button_with_content
      ?key
      ?style
      ?config
      ?disabled
      ~accessible_name
      ~on_click:(fun () -> on_click)
      content
  ;;

  let command_button_with_content = Gpuio.View.command_button_with_content

  let checkbox
        ?key
        ?style
        ?appearance
        ?tab_order
        ?accessible_name
        ?disabled
        ~state
        ~on_toggle
        text
    =
    Gpuio.View.checkbox
      ?key
      ?style
      ?appearance
      ?tab_order
      ?accessible_name
      ?disabled
      ~state
      ~on_toggle:(fun () -> on_toggle)
      text
  ;;

  let switch
        ?key
        ?style
        ?appearance
        ?tab_order
        ?accessible_name
        ?disabled
        ~checked
        ~on_toggle
        text
    =
    Gpuio.View.switch
      ?key
      ?style
      ?appearance
      ?tab_order
      ?accessible_name
      ?disabled
      ~checked
      ~on_toggle:(fun () -> on_toggle)
      text
  ;;

  let checkbox_with_label
        ?key
        ?style
        ?appearance
        ?tab_order
        ?disabled
        ~accessible_name
        ~state
        ~on_toggle
        label
    =
    Gpuio.View.checkbox_with_label
      ?key
      ?style
      ?appearance
      ?tab_order
      ?disabled
      ~accessible_name
      ~state
      ~on_toggle:(fun () -> on_toggle)
      label
  ;;

  let switch_with_label
        ?key
        ?style
        ?appearance
        ?tab_order
        ?disabled
        ~accessible_name
        ~checked
        ~on_toggle
        label
    =
    Gpuio.View.switch_with_label
      ?key
      ?style
      ?appearance
      ?tab_order
      ?disabled
      ~accessible_name
      ~checked
      ~on_toggle:(fun () -> on_toggle)
      label
  ;;

  let row = Gpuio.View.row
  let title_bar = Gpuio.View.title_bar

  let window_controls
        ?key
        ?style
        ?button_style
        ~backend
        ~snapshot
        ~on_minimize
        ~on_zoom
        ~on_close
        ()
    =
    Gpuio.View.window_controls
      ?key
      ?style
      ?button_style
      ~backend
      ~snapshot
      ~on_minimize:(fun () -> on_minimize)
      ~on_zoom:(fun () -> on_zoom)
      ~on_close:(fun () -> on_close)
      ()
  ;;

  let radio
        ?key
        ?style
        ?appearance
        ?accessible_name
        ?disabled
        ?tab_order
        ?position
        ~checked
        ~on_select
        text
    =
    Gpuio.View.radio
      ?key
      ?style
      ?appearance
      ?accessible_name
      ?disabled
      ?tab_order
      ?position
      ~checked
      ~on_select:(fun () -> on_select)
      text
  ;;

  let radio_with_label
        ?key
        ?style
        ?appearance
        ?disabled
        ?tab_order
        ?position
        ~accessible_name
        ~checked
        ~on_select
        label
    =
    Gpuio.View.radio_with_label
      ?key
      ?style
      ?appearance
      ?disabled
      ?tab_order
      ?position
      ~accessible_name
      ~checked
      ~on_select:(fun () -> on_select)
      label
  ;;

  let focus_scope = Gpuio.View.focus_scope
  let dialog = Gpuio.View.dialog
  let sheet = Gpuio.View.sheet
  let alert_dialog = Gpuio.View.alert_dialog
  let popover = Gpuio.View.popover
  let command_scope = Gpuio.View.command_scope
  let slider = Gpuio.View.slider
  let rating = Gpuio.View.rating
  let avatar = Gpuio.View.avatar
  let avatar_with_fallback = Gpuio.View.avatar_with_fallback
  let loading = Gpuio.View.loading
  let spinner = Gpuio.View.spinner
  let progress = Gpuio.View.progress
  let progress_circle = Gpuio.View.progress_circle
  let command_palette = Gpuio.View.command_palette
  let with_palette_content = Gpuio.View.with_palette_content
  let split_button = Gpuio.View.split_button
  let menu_button = Gpuio.View.menu_button
  let context_menu = Gpuio.View.context_menu
  let number_frame = Gpuio.View.number_frame
  let input_frame = Gpuio.View.input_frame
  let editor_menu = Gpuio.View.editor_menu
  let menu_bar = Gpuio.View.menu_bar
  let with_menu_item_content = Gpuio.View.with_menu_item_content
  let command_button = Gpuio.View.command_button
  let tooltip = Gpuio.View.tooltip
  let hover_card = Gpuio.View.hover_card
  let extension = Gpuio.View.extension
  let split_group = Gpuio.View.split_group
  let split_pane = Gpuio.View.split_pane
  let tab_bar = Gpuio.View.tab_bar
  let tab_bar_frame = Gpuio.View.tab_bar_frame

  module Tab_content = Gpuio.View.Tab_content

  let tab_bar_with_content = Gpuio.View.tab_bar_with_content
  let tab_bar_with_labels = Gpuio.View.tab_bar_with_labels
  let tab_panel = Gpuio.View.tab_panel
  let panel = Gpuio.View.panel
  let carousel_track = Gpuio.View.carousel_track
  let carousel = Gpuio.View.carousel
  let navigation_stack = Gpuio.View.navigation_stack
  let accordion = Gpuio.View.accordion
  let accordion_with_labels = Gpuio.View.accordion_with_labels
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
        ?motion
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
      ?motion
      ~on_toggle:(fun () -> on_toggle)
      children
  ;;

  let radio_group = Gpuio.View.radio_group
  let radio_group_with_labels = Gpuio.View.radio_group_with_labels
  let select = Gpuio.View.select
  let choice_picker = Gpuio.View.choice_picker
  let combobox = Gpuio.View.combobox
  let with_list_input = Gpuio.View.with_list_input
  let virtual_list = Gpuio.View.virtual_list
  let column = Gpuio.View.column
  let grid = Gpuio.View.grid
end
