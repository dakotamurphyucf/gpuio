module Ui_command = Command
open Core

module Kind = struct
  type t =
    | Container
    | Text
    | Button
    | Input
    | Textarea
    | Checkbox
    | Switch
    | Radio_group
    | Select
    | Combobox
    | Choice_picker
    | Focus_scope
    | Tooltip
    | Command_scope
    | Command_button
    | Menu
    | Command_palette
    | Progress
    | Toast
    | Toast_stack
    | Pointer_area
    | Drag_source
    | Drop_target
    | Image
    | Icon
    | Animated
    | Virtual_list
    | Document_view
    | Tab_bar
    | Tab_panel
    | Split_pane
    | Split_group
    | Extension
    | Canvas_view
    | Animation_program
    | Container_query
    | Loading
    | Avatar
    | Rating
    | Slider
    | Number_input
    | Otp_input
    | Calendar
    | Color_input
    | Panel
    | Disclosure
    | Accordion
    | Navigation_stack
    | Hover_card
    | Carousel
    | Carousel_track
    | Carousel_track_group
    | Chart_view
    | Input_region
    | Highlight_scope
    | Link
    | Radio
  [@@deriving equal, sexp_of]
end

module Control = struct
  type t =
    | Button of { disabled : bool }
    | Checkbox of
        { state : Check_state.t
        ; disabled : bool
        }
    | Switch of
        { checked : bool
        ; disabled : bool
        }
    | Radio of
        { checked : bool
        ; disabled : bool
        ; position : Radio.Position.t option
        }
  [@@deriving equal, sexp_of]

  let to_wire = function
    | Button { disabled } -> Gpuio_protocol.Wire.Control.Button disabled
    | Checkbox { state; disabled } ->
      let state =
        match state with
        | Check_state.Unchecked -> Gpuio_protocol.Wire.Check_state.Unchecked
        | Checked -> Checked
        | Indeterminate -> Indeterminate
      in
      Checkbox (state, disabled)
    | Switch { checked; disabled } -> Switch (checked, disabled)
    | Radio { checked; disabled; position } ->
      Radio (checked, Option.map position ~f:Radio.Position.Expert.to_wire, disabled)
  ;;
end

type 'action editor_callback =
  | Editor_events of (Text_input.Event.t -> 'action)
  | Picker_query

type 'action editor =
  { controller : Key.t
  ; config : Text_input.Config.t
  ; frame : Input_frame.t option
  ; on_event : 'action editor_callback
  }

type 'action slider =
  { controller : Key.t
  ; config : Slider.Config.t
  ; appearance : Slider.Appearance.t option
  ; initial : Slider.Value.t
  ; on_event : Slider.Event.t -> 'action
  }

type 'action number_input =
  { controller : Key.t
  ; config : Number_input.Config.t
  ; appearance : Number_input.Appearance.t option
  ; initial : Number_input.Value.t
  ; initial_draft : Number_input.Draft.t option
  ; on_event : Number_input.Event.t -> 'action
  }

type 'action otp_input =
  { controller : Key.t
  ; appearance : Otp_input.Appearance.t option
  ; config : Otp_input.Config.t
  ; initial : Otp_input.Value.t
  ; on_event : Otp_input.Event.t -> 'action
  }

type 'action color_input =
  { controller : Key.t
  ; config : Color_input.Config.t
  ; appearance : Color_input.Appearance.t option
  ; initial : Color_value.Value.t
  ; on_event : Color_input.Event.t -> 'action
  }

type 'action calendar =
  { controller : Key.t
  ; config : Calendar.Config.t
  ; initial : Calendar.Selection.t
  ; initial_month : Calendar.Month.t
  ; appearance : Calendar.Appearance.t option
  ; content : Gpuio_protocol.Calendar_content_wire.t option
  ; on_event : Calendar.Event.t -> 'action
  ; on_viewport_change : (Calendar.Viewport.t -> 'action) option
  }

type 'action rating =
  { config : Rating.Config.t
  ; appearance : Rating.Appearance.t option
  ; on_request : Rating.Request.t -> 'action
  }

type 'action choice =
  { config : Choice.Config.t
  ; appearance : Choice.Appearance.t option
  ; tab_appearance : Tab_bar.Appearance.t option
  ; tab_content : Gpuio_protocol.Wire.Tab_content.t option
  ; tab_viewport : Tab_bar.Viewport.t option
  ; tab_motion : Tab_bar.Motion.t option
  ; tab_trailing : bool
  ; choice_menu : bool
  ; on_select : Choice.Id.t -> 'action
  }

type 'action combobox =
  { controller : Key.t
  ; config : Combobox.Config.t
  ; appearance : Choice.Appearance.t
  ; on_event : Combobox.Event.t -> 'action
  }

type 'action overlay =
  { kind : Gpuio_protocol.Wire.Overlay_kind.t
  ; config : Overlay.Config.t
  ; backdrop : Color.t option
  ; motion : Overlay.Motion.t
  ; sheet_insets : Sheet.Insets.t option
  ; on_dismiss : Overlay.Dismissal.t -> 'action
  }

type 'action tooltip =
  { config : Tooltip.Config.t
  ; on_open_change : (bool -> 'action) option
  }

type 'action menu =
  { presentation : Menu.Expert.presentation
  ; menus : Menu.t list
  ; appearance : Menu.Appearance.t
  ; placement : Placement.t option
  ; on_open_change : (bool -> 'action) option
  }

type 'action palette =
  { config : Command_palette.Config.t
  ; appearance : Command_palette.Appearance.t
  ; on_dismiss : Command_palette.Dismissal.t -> 'action
  }

type 'action drag_source =
  { config : Drag_and_drop.Source.t
  ; on_event : Drag_and_drop.Source_event.t -> 'action
  }

type 'action drop_target =
  { config : Drag_and_drop.Target.t
  ; on_event : Drag_and_drop.Target_event.t -> 'action
  }

type 'action command_binding_scope =
  { config : Command_binding.Config.t
  ; on_update : Command_binding.Observation.t -> 'action
  }

type 'action highlight_scope =
  { config : Highlight.Config.t
  ; on_update : (Highlight.Observation.t -> 'action) option
  }

type 'action input_region =
  { config : Input_region.Config.t
  ; on_event : Input_region.Event.t -> 'action
  }

type 'action pointer =
  { config : Pointer.Config.t
  ; on_event : Pointer.Event.t -> 'action
  }

type 'action notification =
  { config : Toast.Config.t
  ; on_dismiss : Toast.Dismissal.t -> 'action
  }

type 'action container_query =
  { config : Container_query.Config.t
  ; on_select : (Container_query.Selection.t -> 'action) option
  }

type 'action animation_program =
  { config : Animation.Program.t
  ; on_event : (Animation.Program.Event.t -> 'action) option
  }

type 'action animation =
  { config : Animation.Config.t
  ; on_event : (Animation.Event.t -> 'action) option
  }

type 'action extension =
  { config : Gpuio_protocol.Extension_wire.Config.t
  ; on_event : Gpuio_protocol.Extension_wire.Signal.t -> 'action
  }

type 'action split_group =
  { config : Split_group.Config.t
  ; appearance : Split_group.Appearance.t
  ; on_resize : (Split_group.Snapshot.t -> 'action) option
  }

type 'action split_pane =
  { config : Split_pane.Config.t
  ; on_resize : (Split_pane.Snapshot.t -> 'action) option
  }

type 'action canvas =
  { config : Canvas.Config.t
  ; on_event : (Canvas.Event.t -> 'action) option
  }

type 'action chart =
  { config : Chart.Config.t
  ; on_event : (Chart.Event.t -> 'action) option
  }

type 'action document =
  { config : Document.Config.t
  ; on_navigate : (Document.Navigation.t -> 'action) option
  ; on_diff : (Document.Diff.Event.t -> 'action) option
  ; on_preview : (Document.Preview.Event.t -> 'action) option
  ; on_action : (Document.Actions.Event.t -> 'action) option
  ; inherit_profile : bool
  ; profile :
      (Gpuio_protocol.Document_profile_wire.Instance.t
      * (Gpuio_protocol.Document_profile_wire.Event.t -> 'action option))
        option
  }

type 'action image =
  { config : Image.Config.t
  ; on_change : (Image.State.t -> 'action) option
  }

type 'action table =
  { source_key : Key.t option
  ; config : Table.Config.t
  ; query_generation : int64
  ; commands : Key.t Table.Command.t list
  ; on_input : Key.t Table.Request.t -> 'action
  ; on_column_viewport : (Table.Column_viewport.t -> 'action) option
  }

type 'action virtual_list =
  { config : Virtual_list.Config.t
  ; order : Virtual_list.Order.t
  ; managed : bool
  ; invalidated : Key.t list
  ; invalidation_revision : int64
  ; scroll : Virtual_list.Scroll_request.t option
  ; on_viewport : (Virtual_list.Viewport.t -> 'action) option
  ; on_retain : (Key.t list -> 'action) option
  ; on_tree_input : (Key.t Tree_input.t -> 'action) option
  ; list_input : (List_input.Config.t * (Key.t List_input.t -> 'action)) option
  ; tree_moves : bool
  ; table : 'action table option
  }

type 'action t =
  { key : Key.t option
  ; structural_key : (string * string) option
  ; kind : Kind.t
  ; text : string
  ; text_content : Text_content.t option
  ; text_shimmer : Text_shimmer.Config.t option
  ; scrollbar : Scrollbar.t option
  ; window_region : Window_region.t option
  ; link : Link.Config.t option
  ; style : Style.t
  ; on_click : (unit -> 'action) option
  ; on_hover : (bool -> 'action) option
  ; editor : 'action editor option
  ; control : Control.t option
  ; split_button :
      (Split_button.Appearance.t * Gpuio_protocol.Wire.Split_button.Parts.t) option
  ; button_presentation : Button.Expert.Presentation.t option
  ; tab_order : Tab_order.t option
  ; control_appearance : Control_appearance.t option
  ; choice : 'action choice option
  ; combobox : 'action combobox option
  ; choice_picker :
      ('action t Choice_picker.Description.t * (Choice_picker.Event.t -> 'action)) option
  ; popover : bool
  ; overlay : 'action overlay option
  ; tooltip : 'action tooltip option
  ; commands : 'action Ui_command.Registry.t option
  ; command_ref : Ui_command.Id.t option
  ; drag_source : 'action drag_source option
  ; drop_target : 'action drop_target option
  ; pointer : 'action pointer option
  ; input_region : 'action input_region option
  ; highlight_scope : 'action highlight_scope option
  ; command_binding_scope : 'action command_binding_scope option
  ; notification : 'action notification option
  ; toast_stack : Toast.Stack.t option
  ; progress : Progress.Config.t option
  ; progress_presentation : Gpuio_protocol.Progress_wire.Presentation.t option
  ; loading : Loading.Config.t option
  ; spinner : Spinner.Config.t option
  ; avatar : Avatar.Config.t option
  ; rating : 'action rating option
  ; slider : 'action slider option
  ; number_input : 'action number_input option
  ; otp_input : 'action otp_input option
  ; color_input : 'action color_input option
  ; calendar : 'action calendar option
  ; animation : 'action animation option
  ; animation_program : 'action animation_program option
  ; reveal : Gpuio_protocol.Reveal_wire.t option
  ; navigation_stack : Gpuio_protocol.Navigation_stack_wire.Config.t option
  ; carousel :
      (Gpuio_protocol.Carousel_wire.Config.t * (Carousel.Request.t -> 'action)) option
  ; carousel_track_motion : Gpuio_protocol.Carousel_track_wire.Motion.t option
  ; carousel_track :
      (Gpuio_protocol.Carousel_track_wire.Config.t
      * (Carousel_track.Request.t -> 'action))
        option
  ; container_query : 'action container_query option
  ; accessibility : Accessibility.t option
  ; image : 'action image option
  ; extension : 'action extension option
  ; split_pane : 'action split_pane option
  ; split_group : 'action split_group option
  ; document : 'action document option
  ; canvas : 'action canvas option
  ; chart : 'action chart option
  ; palette : 'action palette option
  ; menu : 'action menu option
  ; focus_scope : Focus_scope.t option
  ; virtual_list : 'action virtual_list option
  ; table_header : Table_header.Target.t option
  ; table_header_style : Table_presentation.Header.t option
  ; table_row_style : Table_presentation.Row.t option
  ; table_cell : Table.Cell.t option
  ; children : 'action t list
  }

type 'action toast = Toast_item of 'action t

let with_key t key = { t with key = Some key }

let with_window_region t window_region =
  match t.kind with
  | Container -> Ok { t with window_region }
  | _ -> Or_error.error_string "native window regions require an ordinary container"
;;

let with_scrollbar t scrollbar =
  match t.kind with
  | Container | Virtual_list -> Ok { t with scrollbar }
  | _ ->
    Or_error.error_string
      "scrollbar presentation requires a container or managed list/tree/table root"
;;

let text ?key ?(style = Style.empty) text =
  { key
  ; structural_key = None
  ; kind = Text
  ; text
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style
  ; on_hover = None
  ; on_click = None
  ; editor = None
  ; choice = None
  ; combobox = None
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; split_button = None
  ; button_presentation = None
  ; tab_order = None
  ; control_appearance = None
  ; control = None
  ; children = []
  }
;;

let styled_text ?key ?style content =
  { (text ?key ?style (Text_content.text content)) with text_content = Some content }
;;

let with_text_shimmer t text_shimmer =
  if not (Kind.equal t.kind Text)
  then Or_error.error_string "text shimmer requires ordinary text"
  else if
    Option.is_some text_shimmer
    && (String.length t.text > Gpuio_protocol.Text_shimmer_wire.max_text_bytes
        || not (Stdlib.String.is_valid_utf_8 t.text))
  then Or_error.error_string "text shimmer requires valid UTF-8 of at most 16384 bytes"
  else Ok { t with text_shimmer }
;;

let animate_program ?key ?(style = Style.empty) ?on_event config children =
  { (text ?key ~style "") with
    kind = Animation_program
  ; animation_program = Some { config; on_event }
  ; children
  }
;;

let with_hover t ~on_change =
  match t.kind with
  | Button | Command_button | Link -> Ok { t with on_hover = Some on_change }
  | _ ->
    Or_error.error_string
      "hover observation requires a Button, Command_button or Link root"
;;

let with_accessibility t accessibility =
  let metadata = Accessibility.Expert.to_wire accessibility in
  let supported =
    match metadata.field, metadata.role, t.kind with
    | ( Some _
      , None
      , ( Input
        | Textarea
        | Combobox
        | Checkbox
        | Switch
        | Radio
        | Radio_group
        | Select
        | Rating
        | Slider
        | Number_input
        | Otp_input
        | Calendar
        | Color_input ) ) -> true
    | None, Some Link, (Button | Command_button | Link) -> true
    | None, Some (Navigation | Toolbar _ | Radio_group _), Container -> true
    | None, Some (Tree _ | List_box _), Virtual_list -> true
    | None, Some Log, (Container | Virtual_list) -> true
    | None, Some (Tree_item _ | Option_item _), Container -> true
    | ( None
      , Some
          ( Table _
          | Row_group
          | Table_row _
          | Table_cell _
          | Column_header _
          | Row_header _
          | Caption )
      , Container ) -> true
    | ( None
      , Some
          ( Group
          | Label
          | Separator
          | Description_list
          | Term
          | Definition
          | Status
          | Alert
          | Image
          | Heading _ )
      , (Container | Text) ) -> true
    | ( None
      , None
      , ( Container
        | Virtual_list
        | Text
        | Button
        | Link
        | Command_button
        | Input
        | Textarea
        | Combobox
        | Checkbox
        | Switch
        | Radio
        | Radio_group
        | Select
        | Rating
        | Slider
        | Number_input
        | Otp_input
        | Calendar
        | Color_input ) ) -> true
    | _ -> false
  in
  let current_supported =
    Option.is_none metadata.current
    ||
    match t.kind with
    | Text | Button | Command_button | Link -> true
    | _ -> false
  in
  if supported && current_supported
  then Ok { t with accessibility = Some accessibility }
  else Or_error.error_string "accessibility metadata is incompatible with this view kind"
;;

let animate ?key ?(style = Style.empty) ?on_event config children =
  { (text ?key ~style "") with
    kind = Animated
  ; animation = Some { config; on_event }
  ; children
  }
;;

let image ?key ?(style = Style.empty) ?on_change config =
  { (text ?key ~style "") with kind = Image; image = Some { config; on_change } }
;;

let chart ?key ?(style = Style.empty) ?on_event config =
  { (text ?key ~style "") with kind = Chart_view; chart = Some { config; on_event } }
;;

let canvas ?key ?(style = Style.empty) ?on_event config =
  { (text ?key ~style "") with kind = Canvas_view; canvas = Some { config; on_event } }
;;

let document
      ?key
      ?(style = Style.empty)
      ?on_navigate
      ?on_diff
      ?on_preview
      ?on_action
      config
  =
  { (text ?key ~style "") with
    kind = Document_view
  ; document =
      Some
        { config
        ; on_navigate
        ; on_diff
        ; on_preview
        ; on_action
        ; profile = None
        ; inherit_profile = true
        }
  }
;;

let without_document_profile t =
  match t.document with
  | None -> Or_error.error_string "document profile requires a DocumentView"
  | Some document ->
    Ok
      { t with document = Some { document with profile = None; inherit_profile = false } }
;;

let with_document_profile t instance ~on_event =
  match t.document with
  | None -> Or_error.error_string "document profile requires a DocumentView"
  | Some document ->
    if
      not
        (Document.Mode.equal (Document.Config.mode document.config) Markdown
         || Document.Mode.equal (Document.Config.mode document.config) Html)
    then Or_error.error_string "document profile requires Markdown or HTML"
    else (
      let on_event event =
        Document.Profile.Expert.event instance event
        |> Result.ok
        |> Option.map ~f:on_event
      in
      Ok
        { t with
          document =
            Some
              { document with
                profile = Some (Document.Profile.Expert.to_wire instance, on_event)
              ; inherit_profile = false
              }
        })
;;

let icon ?key ?style ?on_change config =
  { (image ?key ?style ?on_change (Icon.Expert.image config)) with kind = Icon }
;;

let container ?key ?(style = Style.empty) defaults children =
  { key
  ; structural_key = None
  ; kind = Container
  ; text = ""
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style = Style.merge [ Style.create_exn defaults; style ]
  ; on_hover = None
  ; on_click = None
  ; editor = None
  ; choice = None
  ; combobox = None
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; split_button = None
  ; button_presentation = None
  ; tab_order = None
  ; control_appearance = None
  ; control = None
  ; children
  }
;;

let icon_slots leading trailing =
  match leading, trailing with
  | None, None -> []
  | _ ->
    List.map
      [ "leading-icon", leading; "trailing-icon", trailing ]
      ~f:(fun (key, decoration) ->
        let children =
          Option.to_list decoration
          |> List.map ~f:(fun decoration ->
            let config, style = Icon.Expert.decoration decoration in
            icon ~style config)
        in
        container ~key:(Key.of_string_exn key) [] children)
;;

let icon_button_style style children =
  if List.is_empty children
  then style
  else
    Style.merge
      [ Style.create_exn
          [ Display Flex; Direction Row; Align_items Center; Gap (Length.px_exn 8.) ]
      ; style
      ]
;;

let button_style style =
  let defaults =
    Style.create_exn
      [ Padding (Length.px_exn 8.)
      ; Radius 4.
      ; Background (Background.solid (Color.token_exn "accent"))
      ; Foreground (Color.token_exn "foreground")
      ; Cursor Pointer
      ; Border_width 1.
      ; Border_color (Color.token_exn "accent")
      ]
    |> fun t ->
    Style.with_state_exn t Focused [ Border_color (Color.token_exn "foreground") ]
  in
  Style.merge [ defaults; style ]
;;

let button
      ?key
      ?(style = Style.empty)
      ?config
      ?accessible_name
      ?(disabled = false)
      ?leading_icon
      ?trailing_icon
      ~on_click
      text
  =
  let children = icon_slots leading_icon trailing_icon in
  let style = icon_button_style style children in
  let style =
    match accessible_name with
    | None -> style
    | Some name -> Style.merge [ style; Style.create_exn [ Accessible_name name ] ]
  in
  { key
  ; structural_key = None
  ; kind = Button
  ; text
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style = button_style style
  ; on_hover = None
  ; on_click =
      (if disabled || Option.exists config ~f:Button.Config.is_loading
       then None
       else Some on_click)
  ; editor = None
  ; choice = None
  ; combobox = None
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; split_button = None
  ; button_presentation = Option.map config ~f:Button.Expert.Presentation.icon_slots
  ; tab_order = None
  ; control_appearance = None
  ; control = Some (Button { disabled })
  ; children
  }
;;

let icon_button ?key ?style ?config ?disabled ~label ~on_click icon =
  button
    ?key
    ?style
    ?config
    ?disabled
    ~accessible_name:label
    ~leading_icon:icon
    ~on_click
    ""
;;

let toggle
      ?key
      ?appearance
      ?tab_order
      ?(style = Style.empty)
      ?accessible_name
      ~disabled
      ~control
      ~kind
      ~on_toggle
      text
  =
  let style =
    match accessible_name with
    | None -> style
    | Some name -> Style.merge [ style; Style.create_exn [ Accessible_name name ] ]
  in
  let defaults =
    Style.create_exn
      [ Display Flex
      ; Direction Row
      ; Align_items Center
      ; Column_gap
          (Length.px_exn
             (Option.value_map appearance ~default:8. ~f:Control_appearance.gap))
      ; Padding (Length.px_exn 6.)
      ; Radius 4.
      ; Border_width 1.
      ; Border_color (Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> Or_error.ok_exn)
      ; Foreground (Color.token_exn "foreground")
      ]
    |> fun t -> Style.with_state_exn t Focused [ Border_color (Color.token_exn "accent") ]
  in
  { key
  ; structural_key = None
  ; kind
  ; text
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style = Style.merge [ defaults; style ]
  ; on_hover = None
  ; on_click = (if disabled then None else Some on_toggle)
  ; editor = None
  ; choice = None
  ; combobox = None
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; split_button = None
  ; button_presentation = None
  ; tab_order
  ; control_appearance = appearance
  ; control = Some control
  ; children = []
  }
;;

let checkbox
      ?key
      ?style
      ?appearance
      ?tab_order
      ?accessible_name
      ?(disabled = false)
      ~state
      ~on_toggle
      text
  =
  toggle
    ?key
    ?appearance
    ?tab_order
    ?style
    ?accessible_name
    ~disabled
    ~control:(Checkbox { state; disabled })
    ~kind:Checkbox
    ~on_toggle
    text
;;

let switch
      ?key
      ?style
      ?appearance
      ?tab_order
      ?accessible_name
      ?(disabled = false)
      ~checked
      ~on_toggle
      text
  =
  toggle
    ?key
    ?appearance
    ?tab_order
    ?style
    ?accessible_name
    ~disabled
    ~control:(Switch { checked; disabled })
    ~kind:Switch
    ~on_toggle
    text
;;

let radio
      ?key
      ?style
      ?appearance
      ?accessible_name
      ?(disabled = false)
      ?tab_order
      ?position
      ~checked
      ~on_select
      text
  =
  let view =
    toggle
      ?key
      ?style
      ?appearance
      ?accessible_name
      ?tab_order
      ~disabled
      ~control:(Radio { checked; disabled; position })
      ~kind:Radio
      ~on_toggle:on_select
      text
  in
  if checked then { view with on_click = None } else view
;;

let focus_scope ?key ?style ~config children =
  { (container ?key ?style [] children) with
    kind = Focus_scope
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = Some config
  }
;;

let overlay_style style =
  Style.merge
    [ Style.create_exn
        [ Background (Background.solid (Color.token_exn "background"))
        ; Foreground (Color.token_exn "foreground")
        ; Border_color (Color.token_exn "muted")
        ]
    ; Option.value style ~default:Style.empty
    ]
;;

let modal_overlay
      ?key
      ?style
      ?backdrop
      ?(motion = Overlay.Motion.Immediate)
      ?sheet_insets
      ~kind
      ~config
      ~on_dismiss
      content
  =
  match content with
  | None ->
    container
      ?key
      [ Position Absolute; Width (Length.px_exn 0.); Height (Length.px_exn 0.) ]
      []
  | Some content ->
    { (focus_scope
         ?key
         ~style:(overlay_style style)
         ~config:(Focus_scope.create ~trap:true ())
         [ content ])
      with
      overlay = Some { kind; config; backdrop; motion; sheet_insets; on_dismiss }
    }
;;

let dialog ?key ?style ?backdrop ?motion ~config ~on_dismiss content =
  modal_overlay ?key ?style ?backdrop ?motion ~kind:Dialog ~config ~on_dismiss content
;;

let sheet ?key ?style ?backdrop ?motion ~config ~on_dismiss content =
  modal_overlay
    ?key
    ?style
    ?backdrop
    ?motion
    ?sheet_insets:(Sheet.Expert.insets config)
    ~kind:(Sheet.Expert.kind config)
    ~config:(Sheet.Expert.overlay config)
    ~on_dismiss
    content
;;

let alert_dialog ?key ?style ?backdrop ?motion ~config ~on_dismiss content =
  modal_overlay
    ?key
    ?style
    ?backdrop
    ?motion
    ~kind:Alert_dialog
    ~config:(Alert_dialog.Expert.overlay config)
    ~on_dismiss
    content
;;

let popover ?key ?style ~config ~on_dismiss ~anchor content =
  let children =
    match content with
    | None -> [ anchor ]
    | Some content ->
      let panel =
        { (focus_scope
             ~style:(overlay_style style)
             ~config:(Focus_scope.create ~auto_focus:true ())
             [ content ])
          with
          overlay =
            Some
              { kind = Popover
              ; config
              ; backdrop = None
              ; motion = Immediate
              ; sheet_insets = None
              ; on_dismiss
              }
        }
      in
      [ anchor; panel ]
  in
  { (container ?key [ Position Relative ] children) with popover = true }
;;

let command_scope ?key ?style ~commands children =
  { (container ?key ?style [] children) with
    kind = Command_scope
  ; commands = Some commands
  }
;;

let command_button ?key ?style ?config ?leading_icon ?trailing_icon ~command () =
  let children = icon_slots leading_icon trailing_icon in
  let style =
    button_style (icon_button_style (Option.value style ~default:Style.empty) children)
  in
  { (text ?key ~style "") with
    kind = Command_button
  ; on_hover = None
  ; on_click = None
  ; split_button = None
  ; button_presentation = Option.map config ~f:Button.Expert.Presentation.icon_slots
  ; tab_order = None
  ; control_appearance = None
  ; control = None
  ; command_ref = Some command
  ; children
  }
;;

let menu_button
      ?key
      ?(style = Style.empty)
      ?(appearance = Menu.Appearance.default)
      ?placement
      ?on_open_change
      ~menu
      ()
  =
  { (text ?key ~style:(button_style style) "") with
    kind = Menu
  ; menu =
      Some
        { presentation = Button; menus = [ menu ]; appearance; placement; on_open_change }
  }
;;

let context_menu
      ?key
      ?(style = Style.empty)
      ?(appearance = Menu.Appearance.default)
      ~menu
      child
  =
  { (text ?key ~style "") with
    kind = Menu
  ; menu =
      Some
        { presentation = Context
        ; menus = [ menu ]
        ; appearance
        ; placement = None
        ; on_open_change = None
        }
  ; children = [ child ]
  }
;;

let menu_bar
      ?key
      ?(style = Style.empty)
      ?(appearance = Menu.Appearance.default)
      ?(platform = true)
      menus
  =
  let%map.Or_error () =
    if platform
    then Menu.Expert.validate_platform_collection menus
    else Menu.Expert.validate_collection menus
  in
  { (text ?key ~style "") with
    kind = Menu
  ; menu =
      Some
        { presentation = (if platform then Platform_bar else Bar)
        ; menus
        ; appearance
        ; placement = None
        ; on_open_change = None
        }
  }
;;

let tooltip ?key ?(style = Style.empty) ~config ?on_open_change ~anchor ~content () =
  { (container ?key ~style:(overlay_style (Some style)) [] [ anchor; content ]) with
    kind = Tooltip
  ; tooltip = Some { config; on_open_change }
  }
;;

let hover_card ?key ?style ~config ?on_open_change ~anchor ~content () =
  { (tooltip
       ?key
       ?style
       ~config:(Hover_card.Expert.tooltip config)
       ?on_open_change
       ~anchor
       ~content
       ())
    with
    kind = Hover_card
  }
;;

let extension ?key ?(style = Style.empty) ~on_event instance =
  { (text ?key ~style "") with
    kind = Extension
  ; extension =
      Some
        { config = Extension.Instance.Expert.to_wire instance
        ; on_event =
            (fun signal -> on_event (Extension.Instance.Expert.event instance signal))
        }
  }
;;

let split_pane ?key ?(style = Style.empty) ?on_resize ~config ~first ~second () =
  { (container ?key ~style [] [ first; second ]) with
    kind = Split_pane
  ; split_pane = Some { config; on_resize }
  }
;;

let container_query ?key ?(style = Style.empty) ?on_select config presentations =
  let expected = Container_query.Config.branches config in
  if
    List.length presentations <> List.length expected
    || List.contains_dup
         (List.map presentations ~f:(fun (id, _) ->
            Container_query.Branch_id.to_string id))
         ~compare:String.compare
  then Or_error.error_string "container query needs exactly one presentation per branch"
  else
    let open Or_error.Let_syntax in
    let%map children =
      List.map expected ~f:(fun id ->
        match
          List.find presentations ~f:(fun (candidate, _) ->
            Container_query.Branch_id.equal candidate id)
        with
        | None -> Or_error.error_string "missing container query presentation"
        | Some (_, child) ->
          let key = Key.of_string_exn (Container_query.Branch_id.to_string id) in
          Ok
            (container
               ~key
               [ Width (Length.percent_exn 100.); Height (Length.percent_exn 100.) ]
               [ child ]))
      |> Or_error.all
    in
    { (container ?key ~style [] children) with
      kind = Container_query
    ; container_query = Some { config; on_select }
    }
;;

let row ?key ?style children =
  container ?key ?style [ Display Flex; Direction Row ] children
;;

let title_bar ?key ?style ~backend ~fullscreen children =
  let inset =
    match backend with
    | Window.Backend.Macos when not fullscreen -> 80.
    | Macos | Wayland | X11 -> 12.
  in
  { (container
       ?key
       ?style
       [ Display Flex
       ; Direction Row
       ; Align_items Center
       ; Shrink 0.
       ; Min_height (Length.px_exn 34.)
       ; Padding_left (Length.px_exn inset)
       ]
       children)
    with
    window_region = Some Title_bar
  }
;;

let window_controls
      ?key
      ?style
      ?button_style
      ~backend
      ~(snapshot : Window.Snapshot.t)
      ~on_minimize
      ~on_zoom
      ~on_close
      ()
  =
  let controls = snapshot.presentation.controls in
  let children =
    match backend, snapshot.presentation.decorations with
    | Window.Backend.Macos, _ | (X11 | Wayland), Server -> []
    | (X11 | Wayland), Client _ ->
      let part name label on_click =
        button ~key:(Key.of_string_exn name) ?style:button_style ~on_click label
      in
      List.filter_opt
        [ (if controls.minimize
           then Some (part "minimize" "Minimize" on_minimize)
           else None)
        ; (if controls.maximize
           then
             Some
               (part
                  "maximize"
                  (if snapshot.maximized then "Restore" else "Maximize")
                  on_zoom)
           else None)
        ; Some (part "close" "Close" on_close)
        ]
  in
  row ?key ?style children
;;

let split_button
      ?key
      ?style
      ?(appearance = Split_button.Appearance.default)
      ?primary
      ?menu
      ()
  =
  let open Or_error.Let_syntax in
  let is_split = Option.is_some primary && Option.is_some menu in
  let rec prepare depth ~menu_part part =
    let finish part =
      if not is_split
      then Ok part
      else (
        let join =
          if menu_part
          then
            [ Style.Property.Top_left_radius 0.
            ; Bottom_left_radius 0.
            ; Border_left_width 0.
            ]
          else [ Style.Property.Top_right_radius 0.; Bottom_right_radius 0. ]
        in
        Ok { part with style = Style.merge [ part.style; Style.create_exn join ] })
    in
    match part.kind, part.children with
    | Tooltip, [ anchor; content ] when depth < 8 ->
      let%map anchor = prepare (depth + 1) ~menu_part anchor in
      { part with children = [ anchor; content ] }
    | (Button | Command_button), _ when not menu_part -> finish part
    | Menu, _
      when menu_part
           && Option.exists part.menu ~f:(fun menu ->
             match menu.presentation with
             | Button -> true
             | Context | Bar | Platform_bar | Editor_context -> false) -> finish part
    | _ ->
      Or_error.error_string
        "split button requires button/menu-button parts with at most eight tooltip \
         anchors"
  in
  let slot name ~menu_part part =
    let%map part = prepare 0 ~menu_part part in
    container
      ~key:(Key.of_string_exn name)
      [ Display Flex; Direction Row; Align_items Stretch ]
      [ part ]
  in
  let%bind parts, children =
    match primary, menu with
    | None, None -> Or_error.error_string "split button requires at least one part"
    | Some primary, None ->
      let%map primary = slot "primary" ~menu_part:false primary in
      Gpuio_protocol.Wire.Split_button.Parts.Primary, [ primary ]
    | None, Some menu ->
      let%map menu = slot "menu" ~menu_part:true menu in
      Gpuio_protocol.Wire.Split_button.Parts.Menu, [ menu ]
    | Some primary, Some menu ->
      let%bind primary = slot "primary" ~menu_part:false primary in
      let%map menu = slot "menu" ~menu_part:true menu in
      Gpuio_protocol.Wire.Split_button.Parts.Split, [ primary; menu ]
  in
  Ok
    { (container
         ?key
         ?style
         [ Display Flex
         ; Direction Row
         ; Align_items Stretch
         ; Align_self Start
         ; Gap (Length.px_exn 0.)
         ]
         children)
      with
      split_button = Some (appearance, parts)
    }
;;

let validate_passive_children ?(allow_progress = false) ~context ~validate_style children =
  let rec validate count = function
    | [] -> Ok ()
    | (child, depth) :: rest ->
      let open Or_error.Let_syntax in
      if count >= 4096 || depth > 128
      then Or_error.errorf "%s exceeds 4096 nodes or 128 levels" context
      else if
        ((not
            (List.mem
               [ Kind.Container
               ; Text
               ; Image
               ; Icon
               ; Avatar
               ; Loading
               ; Animated
               ; Animation_program
               ]
               child.kind
               ~equal:Kind.equal))
         && not (allow_progress && Kind.equal child.kind Progress))
        || Option.is_some child.on_click
        || Option.is_some child.on_hover
        || Option.is_some child.window_region
        || Option.is_some child.command_binding_scope
        || Option.exists child.image ~f:(fun image -> Option.is_some image.on_change)
        || Option.exists child.animation ~f:(fun animation ->
          Option.is_some animation.on_event)
        || Option.exists child.animation_program ~f:(fun animation ->
          Option.is_some animation.on_event)
      then Or_error.errorf "%s must be passive and have no callbacks" context
      else (
        let%bind () = validate_style child.style in
        validate
          (count + 1)
          (List.rev_append
             (List.map child.children ~f:(fun child -> child, depth + 1))
             rest))
  in
  validate 0 (List.map children ~f:(fun child -> child, 1))
;;

let validate_control_labels children =
  validate_passive_children
    ~context:"control label"
    ~validate_style:Style.Expert.validate_control_label
    children
;;

let with_menu_item_content t ~items =
  let open Or_error.Let_syntax in
  match t.kind, t.menu with
  | Menu, Some config ->
    let%bind () =
      match config.presentation with
      | Platform_bar ->
        Or_error.error_string "platform menu bars do not support custom content"
      | Button | Context | Bar | Editor_context -> Ok ()
    in
    let path_key path =
      Menu.Item_path.to_list path |> List.map ~f:Int.to_string |> String.concat ~sep:"/"
    in
    let%bind () =
      if List.length items > 1024
      then Or_error.error_string "menu content exceeds 1024 item paths"
      else Ok ()
    in
    let%bind content =
      Map.of_alist_or_error
        (module String)
        (List.map items ~f:(fun (path, view) -> path_key path, view))
    in
    let paths = Menu.Expert.item_paths config.menus in
    let allowed =
      List.filter_map paths ~f:(fun (path, item) ->
        match item with
        | Menu.Item.Separator -> None
        | Command _ | Label _ | Submenu _ -> Some (path_key path))
      |> String.Set.of_list
    in
    let%bind () =
      match List.find (Map.keys content) ~f:(fun key -> not (Set.mem allowed key)) with
      | None -> Ok ()
      | Some key -> Or_error.errorf "unknown or separator menu content path: %s" key
    in
    let slots =
      if Map.is_empty content
      then []
      else
        List.map paths ~f:(fun (path, _) ->
          let name = path_key path in
          { (container [] (Option.to_list (Map.find content name))) with
            style = Style.empty
          ; structural_key = Some ("menu-content", name)
          })
    in
    let%map () = validate_control_labels slots in
    let target =
      match config.presentation with
      | Context | Editor_context -> List.take t.children 1
      | Button | Bar | Platform_bar -> []
    in
    { t with children = target @ slots }
  | _ -> Or_error.error_string "menu item content requires a direct menu view"
;;

let editor_menu
      ?key
      ?style
      ?appearance
      ?(config = Editor_menu.default)
      ?(item_content = [])
      child
  =
  match child.kind, child.editor with
  | (Input | Textarea), Some _ ->
    let key = Option.first_some key child.key in
    let menu = context_menu ?appearance ~menu:(Editor_menu.Expert.menu config) child in
    let menu =
      { menu with
        menu = Option.map menu.menu ~f:(fun m -> { m with presentation = Editor_context })
      }
    in
    let%map.Or_error menu = with_menu_item_content menu ~items:item_content in
    command_scope ?key ?style ~commands:(Editor_menu.Expert.commands config) [ menu ]
  | _ -> Or_error.error_string "editor menu requires one direct text_input view"
;;

let split_group
      ?key
      ?(style = Style.empty)
      ?(appearance = Split_group.Appearance.default)
      ?(handles = [])
      ?on_resize
      ~config
      ~panels
      ()
  =
  let open Or_error.Let_syntax in
  let index values =
    Map.of_alist_or_error
      (module String)
      (List.map values ~f:(fun (id, view) -> Split_group.Id.to_string id, view))
  in
  let%bind panels = index panels in
  let%bind handles = index handles in
  let expected = Split_group.Config.panels config in
  let ids =
    List.map expected ~f:(fun p -> Split_group.Panel.id p |> Split_group.Id.to_string)
  in
  let%bind () =
    if
      Map.length panels <> List.length ids
      || List.exists ids ~f:(fun id -> not (Map.mem panels id))
      || List.exists (Map.keys handles) ~f:(fun id ->
        not (List.mem ids id ~equal:String.equal))
    then
      Or_error.error_string
        "split group requires exactly one content per panel and only known handle IDs"
    else Ok ()
  in
  let%map () = validate_control_labels (Map.data handles) in
  let structural role id children =
    { (container [] children) with
      style = Style.empty
    ; structural_key = Some ("split-group:" ^ role, id)
    }
  in
  let children =
    List.map ids ~f:(fun id ->
      structural
        "panel"
        id
        [ structural "content" id [ Map.find_exn panels id ]
        ; structural "handle" id (Option.to_list (Map.find handles id))
        ])
  in
  { (container ?key ~style [] children) with
    kind = Split_group
  ; split_group = Some { config; appearance; on_resize }
  }
;;

module Calendar_content = struct
  module Wire = Gpuio_protocol.Calendar_content_wire

  type 'action view = 'action t

  let validate children =
    validate_passive_children
      ~allow_progress:true
      ~context:"calendar content"
      ~validate_style:Style.Expert.validate_control_label
      children
  ;;

  module Item = struct
    type 'action t =
      { metadata : Wire.Item.t
      ; content : 'action view
      }

    let create ?description ~slot content =
      let open Or_error.Let_syntax in
      let metadata =
        { Wire.Item.slot = Calendar.Expert.slot_to_wire slot; description }
      in
      let%bind () =
        if Wire.Item.valid metadata
        then Ok ()
        else Or_error.error_string "invalid calendar content description"
      in
      let%map () = validate [ content ] in
      { metadata; content }
    ;;
  end

  type 'action t =
    { metadata : Wire.t
    ; children : 'action view list
    }

  let create items =
    let open Or_error.Let_syntax in
    let%bind () =
      if List.length items <= Wire.max_items
      then Ok ()
      else Or_error.error_string "calendar content exceeds 1024 slots"
    in
    let items =
      List.sort items ~compare:(fun a b ->
        Wire.Slot.compare a.Item.metadata.slot b.Item.metadata.slot)
    in
    let metadata = List.map items ~f:(fun i -> i.Item.metadata) in
    let%bind () =
      if Wire.valid metadata
      then Ok ()
      else
        Or_error.error_string
          "calendar content has duplicate slots or exceeds description limits"
    in
    (* Structural slots must carry no style declarations. [container []] emits
       an empty base declaration, which native slot validation rejects. *)
    let children =
      List.map items ~f:(fun item ->
        { (text "") with
          kind = Container
        ; structural_key = Some ("calendar-content", Wire.Slot.key item.metadata.slot)
        ; children = [ item.Item.content ]
        })
    in
    let%map () = validate children in
    { metadata; children }
  ;;
end

let validate_control_name name =
  if
    Gpuio_protocol.Accessibility_wire.valid_text name
    && String.length name <= 1024
    && not (String.is_empty (String.strip name))
  then Ok ()
  else
    Or_error.error_string "control name must be nonblank UTF-8 without NUL, <=1024 bytes"
;;

let validate_button_content content =
  validate_passive_children
    ~allow_progress:true
    ~context:"button content"
    ~validate_style:Style.Expert.validate_control_label
    [ content ]
;;

let button_with_content
      ?key
      ?style
      ?(config = Button.Config.default)
      ?disabled
      ~accessible_name
      ~on_click
      content
  =
  let open Or_error.Let_syntax in
  let%bind () = validate_control_name accessible_name in
  let%map () = validate_button_content content in
  { (button ?key ?style ~config ?disabled ~on_click accessible_name) with
    button_presentation = Some (Button.Expert.Presentation.rich config)
  ; children = [ content ]
  }
;;

let command_button_with_content
      ?key
      ?style
      ?(config = Button.Config.default)
      ~command
      content
  =
  let%map.Or_error () = validate_button_content content in
  { (command_button ?key ?style ~config ~command ()) with
    button_presentation = Some (Button.Expert.Presentation.rich config)
  ; children = [ content ]
  }
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
  let open Or_error.Let_syntax in
  let%bind () = validate_control_name accessible_name in
  let%map () = validate_control_labels [ label ] in
  { (checkbox
       ?key
       ?style
       ?appearance
       ?tab_order
       ?disabled
       ~accessible_name
       ~state
       ~on_toggle
       accessible_name)
    with
    children = [ label ]
  }
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
  let open Or_error.Let_syntax in
  let%bind () = validate_control_name accessible_name in
  let%map () = validate_control_labels [ label ] in
  { (switch
       ?key
       ?style
       ?appearance
       ?tab_order
       ?disabled
       ~accessible_name
       ~checked
       ~on_toggle
       accessible_name)
    with
    children = [ label ]
  }
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
  let open Or_error.Let_syntax in
  let%bind () = validate_control_name accessible_name in
  let%map () = validate_control_labels [ label ] in
  { (radio
       ?key
       ?style
       ?appearance
       ?disabled
       ?tab_order
       ?position
       ~accessible_name
       ~checked
       ~on_select
       accessible_name)
    with
    children = [ label ]
  }
;;

let link ?key ?style config ~on_click children =
  let%map.Or_error () =
    validate_passive_children
      ~context:"link content"
      ~validate_style:Style.Expert.validate_link_content
      children
  in
  { (container
       ?key
       ?style
       [ Display Flex; Direction Row; Align_items Center; Gap (Length.px_exn 8.) ]
       children)
    with
    kind = Link
  ; link = Some config
  ; on_hover = None
  ; on_click =
      (if Link.Config.is_disabled config || Link.Config.is_loading config
       then None
       else Some on_click)
  }
;;

let column ?key ?style children =
  container ?key ?style [ Display Flex; Direction Column ] children
;;

let tab_panel ~key ~label ~active ?(style = Style.empty) children =
  let style =
    Style.merge
      [ style
      ; Style.create_exn [ Accessible_name label ]
      ; (if active then Style.empty else Style.create_exn [ Display Hidden ])
      ]
  in
  { (column ~key ~style children) with kind = Tab_panel; text = label }
;;

let panel
      ~key
      ~label
      ~active
      ~hidden
      ?(motion = Disclosure.Motion.immediate)
      ?(style = Style.empty)
      children
  =
  if not (Gpuio_protocol.Accessibility_wire.valid_text label)
  then invalid_arg "panel label must be nonempty UTF-8 without NUL, at most 4096 bytes";
  let children =
    match hidden with
    | Content_policy.Retain -> children
    | Unmount -> if active then children else []
  in
  let style =
    Style.merge
      [ style; (if active then Style.empty else Style.create_exn [ Display Hidden ]) ]
  in
  { (column ~key ~style children) with
    kind = Panel
  ; text = label
  ; reveal = Disclosure.Expert.motion_config motion ~expanded:active ~hidden
  }
;;

let navigation_stack
      model
      ?key
      ?style
      ?page_style
      ?(motion = Navigation_stack.Motion.default)
      ~hidden
      ~label
      ~content
      ()
  =
  if not (Gpuio_protocol.Accessibility_wire.valid_text label)
  then
    invalid_arg "navigation label must be nonempty UTF-8 without NUL, at most 4096 bytes";
  let selected =
    Option.map (Navigation_stack.current model) ~f:Navigation_stack.Entry.id
  in
  let children =
    List.map (Navigation_stack.entries model) ~f:(fun entry ->
      let id = Navigation_stack.Entry.id entry in
      let active = Option.exists selected ~f:(Navigation_stack.Id.equal id) in
      let children =
        match hidden with
        | Content_policy.Retain -> content entry
        | Unmount -> if active then content entry else []
      in
      panel
        ~key:(Key.of_string_exn (Navigation_stack.Id.to_string id))
        ~label:(Navigation_stack.Entry.label entry)
        ~active:true
        ~hidden:Content_policy.Retain
        ?style:page_style
        children)
  in
  { (column ?key ?style children) with
    kind = Navigation_stack
  ; text = label
  ; navigation_stack =
      Some (Navigation_stack.Expert.presentation_config model ~hidden ~motion)
  }
;;

let carousel_track
      model
      ?key
      ?(motion = Carousel_track.Motion.default)
      ?style
      ?viewport_style
      ?track_style
      ?item_style
      ?controls_style
      ?control_style
      ?(show_controls = true)
      ~label
      ~on_request
      ~content
      ()
  =
  if not (Gpuio_protocol.Accessibility_wire.valid_text label)
  then
    invalid_arg
      "carousel track label must be nonempty UTF-8 without NUL, at most 4096 bytes";
  let config = Carousel_track.Expert.to_wire model in
  let items =
    List.map (Carousel_track.items model) ~f:(fun item ->
      panel
        ~key:
          (Key.of_string_exn (Carousel_track.Id.to_string (Carousel_track.Item.id item)))
        ~label:(Carousel_track.Item.label item)
        ~active:true
        ~hidden:Content_policy.Retain
        ~style:
          (Style.merge
             [ Style.create_exn [ Shrink 0. ]
             ; Option.value_map item_style ~default:Style.empty ~f:(fun f -> f item)
             ])
        (content item))
  in
  let track =
    let constructor =
      match Carousel_track.axis model with
      | Horizontal -> row
      | Vertical -> column
    in
    constructor ~key:(Key.of_string_exn "track") ?style:track_style items
  in
  let viewport =
    { (column
         ~key:(Key.of_string_exn "viewport")
         ~style:
           (Style.merge
              [ Style.create_exn
                  [ Grow 1.; Min_height (Length.px_exn 0.); Min_width (Length.px_exn 0.) ]
              ; Option.value viewport_style ~default:Style.empty
              ])
         [ track ])
      with
      kind = Carousel_track
    ; text = label
    ; carousel_track_motion = Carousel_track.Motion.Expert.to_wire motion
    ; carousel_track = Some (config, on_request)
    }
  in
  let controls =
    if not show_controls
    then []
    else (
      let control key label enabled request =
        button
          ~key:(Key.of_string_exn key)
          ?style:control_style
          ~disabled:(not enabled)
          ~on_click:(fun () -> on_request request)
          label
      in
      [ row
          ~key:(Key.of_string_exn "controls")
          ~style:
            (Style.merge
               [ Style.create_exn [ Gap (Length.px_exn 6.); Align_items Center ]
               ; Option.value controls_style ~default:Style.empty
               ])
          [ control
              "previous"
              "Previous"
              (Carousel_track.can_previous model)
              Carousel_track.Request.previous
          ; control
              "next"
              "Next"
              (Carousel_track.can_next model)
              Carousel_track.Request.next
          ]
      ])
  in
  { (column ?key ?style (viewport :: controls)) with kind = Carousel_track_group }
;;

let carousel
      model
      ?key
      ?style
      ?viewport_style
      ?page_style
      ?controls_style
      ?control_style
      ?(show_controls = true)
      ?(axis = Carousel.Axis.Horizontal)
      ?(motion = Carousel.Motion.default)
      ~hidden
      ~label
      ~on_request
      ~content
      ()
  =
  if not (Gpuio_protocol.Accessibility_wire.valid_text label)
  then invalid_arg "carousel label must be nonempty UTF-8 without NUL, at most 4096 bytes";
  let config = Carousel.Expert.to_wire model ~axis in
  let children =
    List.mapi (Carousel.items model) ~f:(fun index item ->
      let active = Option.equal Int64.equal config.selected (Some (Int64.of_int index)) in
      let children =
        match hidden with
        | Content_policy.Retain -> content item
        | Unmount -> if active then content item else []
      in
      panel
        ~key:(Key.of_string_exn (Carousel.Id.to_string (Carousel.Item.id item)))
        ~label:(Carousel.Item.label item)
        ~active:true
        ~hidden:Content_policy.Retain
        ?style:page_style
        children)
  in
  let viewport_style =
    Style.merge
      [ Style.create_exn [ Grow 1.; Min_height (Length.px_exn 0.) ]
      ; Option.value viewport_style ~default:Style.empty
      ]
  in
  let viewport =
    { (column ~key:(Key.of_string_exn "viewport") ~style:viewport_style children) with
      kind = Navigation_stack
    ; text = label
    ; navigation_stack =
        Some
          (Navigation_stack.Expert.motion_config motion ~hidden ~selected:config.selected)
    }
  in
  let controls =
    if not show_controls
    then []
    else (
      let current = Option.map config.selected ~f:(fun i -> Int64.to_int_exn i + 1) in
      let pages =
        Pagination.create
          ~total_pages:(List.length config.ids)
          ?current
          ~disabled:config.disabled
          ()
        |> Or_error.ok_exn
      in
      let control key label enabled request =
        button
          ~key:(Key.of_string_exn key)
          ?style:control_style
          ~disabled:(not enabled)
          ~on_click:(fun () -> on_request request)
          label
      in
      let numbered =
        List.map (Pagination.items pages) ~f:(function
          | Pagination.Item.Gap { first = _; last = _ } -> text "…"
          | Page page ->
            let item = List.nth_exn (Carousel.items model) (page - 1) in
            let selected = Option.equal Int.equal current (Some page) in
            let view =
              control
                (Carousel.Id.to_string (Carousel.Item.id item))
                (Int.to_string page)
                (not config.disabled)
                (Carousel.Request.select (Carousel.Item.id item))
            in
            let metadata =
              Accessibility.create
                ?current:(if selected then Some Page else None)
                ?description:(if selected then Some "Current item" else None)
                ()
              |> Or_error.ok_exn
            in
            with_accessibility view metadata |> Or_error.ok_exn)
      in
      [ row
          ~key:(Key.of_string_exn "controls")
          ~style:
            (Style.merge
               [ Style.create_exn [ Gap (Length.px_exn 6.); Align_items Center ]
               ; Option.value controls_style ~default:Style.empty
               ])
          ([ control
               "first"
               "First"
               ((not config.disabled) && Option.exists current ~f:(fun n -> n > 1))
               Carousel.Request.first
           ; control
               "previous"
               "Previous"
               (Carousel.can_previous model)
               Carousel.Request.previous
           ]
           @ [ row
                 ~key:(Key.of_string_exn "pages")
                 ~style:(Style.create_exn [ Gap (Length.px_exn 6.) ])
                 numbered
             ]
           @ [ control "next" "Next" (Carousel.can_next model) Carousel.Request.next
             ; control
                 "last"
                 "Last"
                 ((not config.disabled)
                  && Option.exists current ~f:(fun n -> n < List.length config.ids))
                 Carousel.Request.last
             ])
      ])
  in
  { (column ?key ?style (viewport :: controls)) with
    kind = Carousel
  ; text = label
  ; carousel = Some (config, on_request)
  }
;;

let disclosure
      ?key
      ?style
      ?trigger_style
      ?panel_style
      ~label
      ~expanded
      ?(disabled = false)
      ~hidden
      ?motion
      ~on_toggle
      children
  =
  let trigger =
    button
      ~key:(Key.of_string_exn "trigger")
      ?style:trigger_style
      ~disabled
      ~on_click:on_toggle
      label
  in
  let panel =
    panel
      ~key:(Key.of_string_exn "panel")
      ~label
      ~active:expanded
      ~hidden
      ?motion
      ?style:panel_style
      children
  in
  { (column ?key ?style [ trigger; panel ]) with kind = Disclosure }
;;

let disclosure_with_header
      ?key
      ?style
      ?header_style
      ?panel_style
      ~label
      ~expanded
      ~hidden
      ?motion
      ~header
      ~trigger
      children
  =
  if not (Gpuio_protocol.Accessibility_wire.valid_text label)
  then Or_error.error_string "invalid disclosure region label"
  else (
    match trigger.kind with
    | Button ->
      let trigger = { trigger with key = Some (Key.of_string_exn "trigger") } in
      let header =
        row
          ~key:(Key.of_string_exn "header")
          ~style:
            (Style.merge
               [ Style.create_exn
                   [ Align_items Center
                   ; Gap (Length.px_exn 6.)
                   ; Min_width (Length.px_exn 0.)
                   ]
               ; Option.value header_style ~default:Style.empty
               ])
          [ row
              ~key:(Key.of_string_exn "content")
              ~style:
                (Style.create_exn
                   [ Grow 1.; Min_width (Length.px_exn 0.); Align_items Center ])
              header
          ; trigger
          ]
      in
      let panel =
        panel
          ~key:(Key.of_string_exn "panel")
          ~label
          ~active:expanded
          ~hidden
          ?motion
          ?style:panel_style
          children
      in
      Ok { (column ?key ?style [ header; panel ]) with kind = Disclosure }
    | _ -> Or_error.error_string "disclosure trigger must be a button")
;;

let accordion
      ?key
      ?style
      ?trigger_style
      ?panel_style
      ~model
      ~hidden
      ?motion
      ~on_request
      ~content
      ()
  =
  let children =
    Choice.Collection.to_list (Disclosure.items model)
    |> List.map ~f:(fun item ->
      let id = Choice.id item in
      let expanded = Disclosure.is_expanded model id in
      let children =
        if expanded || Content_policy.equal hidden Retain then content id else []
      in
      disclosure
        ~key:(Key.of_string_exn (Choice.Id.to_string id))
        ?trigger_style
        ?panel_style
        ~label:(Choice.label item)
        ~expanded
        ~disabled:(Disclosure.is_disabled model || Choice.is_disabled item)
        ~hidden
        ?motion
        ~on_toggle:(fun () -> on_request (Disclosure.Request.Toggle id))
        children)
  in
  { (column ?key ?style children) with kind = Accordion }
;;

let accordion_with_labels
      ?key
      ?style
      ?item_style
      ?trigger_style
      ?panel_style
      ?(heading_level = 3)
      ~model
      ~labels
      ~hidden
      ?motion
      ~on_request
      ~content
      ()
  =
  let open Or_error.Let_syntax in
  let%bind heading = Accessibility.create ~role:(Heading heading_level) () in
  let%bind () =
    if List.length labels <= Choice.Collection.max_choices
    then Ok ()
    else Or_error.error_string "too many rich accordion labels"
  in
  let%bind labels =
    Map.of_alist_or_error
      (module String)
      (List.map labels ~f:(fun (id, label) -> Choice.Id.to_string id, label))
  in
  let items = Disclosure.items model in
  let%bind () =
    Map.keys labels
    |> List.map ~f:(fun id ->
      let id = Choice.Id.of_string id |> Or_error.ok_exn in
      if Option.is_some (Choice.Collection.find items id)
      then Ok ()
      else Or_error.errorf "unknown rich accordion label ID: %s" (Choice.Id.to_string id))
    |> Or_error.all_unit
  in
  let%bind () =
    validate_passive_children
      ~allow_progress:true
      ~context:"accordion labels"
      ~validate_style:Style.Expert.validate_control_label
      (Map.data labels)
  in
  let%map children =
    Choice.Collection.to_list items
    |> List.map ~f:(fun item ->
      let id = Choice.id item in
      let expanded = Disclosure.is_expanded model id in
      let disabled = Disclosure.is_disabled model || Choice.is_disabled item in
      let on_click () = on_request (Disclosure.Request.Toggle id) in
      let trigger_style =
        Style.merge
          [ Style.create_exn [ Grow 1.; Min_width (Length.px_exn 0.) ]
          ; Option.value trigger_style ~default:Style.empty
          ]
      in
      let%bind trigger =
        match Map.find labels (Choice.Id.to_string id) with
        | None ->
          Ok
            (button
               ~key:(Key.of_string_exn "trigger")
               ~style:trigger_style
               ~disabled
               ~on_click
               (Choice.label item))
        | Some label ->
          button_with_content
            ~key:(Key.of_string_exn "trigger")
            ~style:trigger_style
            ~accessible_name:(Choice.label item)
            ~disabled
            ~on_click
            label
      in
      let%map header =
        with_accessibility (row ~key:(Key.of_string_exn "header") [ trigger ]) heading
      in
      let children =
        if expanded || Content_policy.equal hidden Retain then content id else []
      in
      let panel =
        panel
          ~key:(Key.of_string_exn "panel")
          ~label:(Choice.label item)
          ~active:expanded
          ~hidden
          ?motion
          ?style:panel_style
          children
      in
      { (column
           ~key:(Key.of_string_exn (Choice.Id.to_string id))
           ?style:(Option.map item_style ~f:(fun f -> f item))
           [ header; panel ])
        with
        kind = Disclosure
      })
    |> Or_error.all
  in
  { (column ?key ?style children) with kind = Accordion }
;;

let make_virtual_list
      ?key
      ?style
      ~config
      ~order
      ~managed
      ~invalidated
      ~invalidation_revision
      ~scroll
      ~on_viewport
      ~on_retain
      ~on_tree_input
      ~tree_moves
      rows
  =
  let keys = List.map rows ~f:(fun (key, _) -> Key.to_string key) in
  if
    Virtual_list.Axis.equal (Virtual_list.Config.axis config) Horizontal
    && (Option.is_some on_tree_input || tree_moves)
  then Core.Or_error.error_string "native tree input requires a vertical list"
  else if
    Set.length (String.Set.of_list keys) <> List.length keys
    || (not (List.for_all rows ~f:(fun (key, _) -> Virtual_list.Order.mem order key)))
    || (not (List.for_all invalidated ~f:(Virtual_list.Order.mem order)))
    || Int64.(
         invalidation_revision < 0L
         || ((not (List.is_empty invalidated)) && invalidation_revision = 0L))
    || (managed && List.length rows > Virtual_list.Config.max_active config)
    || ((not managed) && List.length rows <> Virtual_list.Order.length order)
  then Core.Or_error.error_string "invalid virtual list row set or active-row budget"
  else (
    let row_style = Virtual_list.Expert.row_style config in
    let children =
      List.map rows ~f:(fun (key, view) ->
        match
          Option.bind view.accessibility ~f:(fun a ->
            (Accessibility.Expert.to_wire a).role)
        with
        | Some (Tree_item _ | Option_item _) ->
          { (column ~key ~style:row_style [ { view with accessibility = None } ]) with
            accessibility = view.accessibility
          }
        | None
        | Some
            ( Group
            | Label
            | Link
            | Separator
            | Description_list
            | Term
            | Definition
            | Status
            | Alert
            | Image
            | Heading _
            | Navigation
            | Tree _
            | List_box _
            | Table _
            | Row_group
            | Table_row _
            | Table_cell _
            | Column_header _
            | Row_header _
            | Caption
            | Toolbar _
            | Radio_group _
            | Log ) -> column ~key ~style:row_style [ view ])
    in
    Ok
      { (column ?key ?style children) with
        kind = Virtual_list
      ; virtual_list =
          Some
            { config
            ; order
            ; managed
            ; invalidated
            ; invalidation_revision
            ; scroll
            ; on_viewport
            ; on_retain
            ; on_tree_input
            ; list_input = None
            ; tree_moves
            ; table = None
            }
      })
;;

let virtual_list ?key ?style ?on_viewport ?scroll ~config rows =
  let open Core.Or_error.Let_syntax in
  let%bind order = Virtual_list.Order.create (List.map rows ~f:fst) in
  make_virtual_list
    ?key
    ?style
    ~config
    ~order
    ~managed:false
    ~invalidated:[]
    ~invalidation_revision:0L
    ~scroll
    ~on_viewport
    ~on_retain:None
    ~on_tree_input:None
    ~tree_moves:false
    rows
;;

let with_list_input t ~config ~on_input =
  match t.virtual_list with
  | Some list
    when Option.is_none list.on_tree_input
         && (not list.tree_moves)
         && Option.is_none list.table ->
    let has_role =
      Option.exists t.accessibility ~f:(fun metadata ->
        match (Accessibility.Expert.to_wire metadata).role with
        | Some (List_box _) -> true
        | _ -> false)
    in
    if not has_role
    then Or_error.error_string "list input requires List_box accessibility"
    else if
      Option.exists (List_input.Config.cursor config) ~f:(fun key ->
        not (Virtual_list.Order.mem list.order key))
    then Or_error.error_string "list cursor is absent from logical order"
    else
      Ok { t with virtual_list = Some { list with list_input = Some (config, on_input) } }
  | Some _ | None ->
    Or_error.error_string "list input requires a virtual list without tree or table input"
;;

let grid ?key ?style ~columns children =
  if columns < 1 || columns > 1024
  then Or_error.error_string "grid columns must be in 1..1024"
  else Ok (container ?key ?style [ Display Grid; Grid_columns columns ] children)
;;

let text_input
      ?(style = Style.empty)
      ?(initial_text = "")
      ~controller
      ~config
      ~on_event
      ()
  =
  let open Or_error.Let_syntax in
  let%map () =
    Text_input.validate_text ~mode:(Text_input.Config.mode config) initial_text
  in
  let kind =
    match Text_input.Config.mode config with
    | Single_line -> Kind.Input
    | Multiline -> Textarea
  in
  { key = Some controller
  ; structural_key = None
  ; kind
  ; text = initial_text
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style
  ; on_hover = None
  ; on_click = None
  ; editor = Some { controller; config; frame = None; on_event = Editor_events on_event }
  ; choice = None
  ; combobox = None
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; split_button = None
  ; button_presentation = None
  ; tab_order = None
  ; control_appearance = None
  ; control = None
  ; children = []
  }
;;

let radio_group ?key ?(style = Style.empty) ?appearance ?tab_order ~config ~on_select () =
  { key
  ; structural_key = None
  ; kind = Radio_group
  ; text = ""
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style
  ; on_hover = None
  ; on_click = None
  ; editor = None
  ; split_button = None
  ; button_presentation = None
  ; tab_order
  ; control_appearance = appearance
  ; control = None
  ; choice =
      Some
        { config
        ; appearance = None
        ; tab_appearance = None
        ; tab_viewport = None
        ; tab_motion = None
        ; tab_trailing = false
        ; choice_menu = false
        ; tab_content = None
        ; on_select
        }
  ; combobox = None
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; children = []
  }
;;

let tab_bar ?key ?style ?appearance ?viewport ?motion ~config ~on_select () =
  { (radio_group ?key ?style ~config ~on_select ()) with
    kind = Tab_bar
  ; choice =
      Some
        { config
        ; appearance = None
        ; tab_appearance = appearance
        ; tab_viewport = viewport
        ; tab_motion = motion
        ; tab_trailing = false
        ; choice_menu = false
        ; tab_content = None
        ; on_select
        }
  }
;;

let radio_group_with_labels
      ?key
      ?style
      ?appearance
      ?tab_order
      ~config
      ~labels
      ~on_select
      ()
  =
  let open Or_error.Let_syntax in
  let%bind labels =
    Map.of_alist_or_error
      (module String)
      (List.map labels ~f:(fun (id, label) -> Choice.Id.to_string id, label))
  in
  let options = Choice.Config.options config |> Choice.Collection.to_list in
  let ids =
    String.Set.of_list
      (List.map options ~f:(fun item -> Choice.Id.to_string (Choice.id item)))
  in
  let%bind () =
    match List.find (Map.keys labels) ~f:(fun id -> not (Set.mem ids id)) with
    | None -> Ok ()
    | Some id -> Or_error.errorf "unknown rich choice label ID: %s" id
  in
  let children =
    if Map.is_empty labels
    then []
    else
      List.map options ~f:(fun item ->
        let id = Choice.Id.to_string (Choice.id item) in
        column ~key:(Key.of_string_exn id) (Option.to_list (Map.find labels id)))
  in
  let%map () = validate_control_labels children in
  { (radio_group ?key ?style ?appearance ?tab_order ~config ~on_select ()) with children }
;;

let tab_bar_with_labels
      ?key
      ?style
      ?appearance
      ?viewport
      ?motion
      ~config
      ~labels
      ~on_select
      ()
  =
  let%map.Or_error view =
    radio_group_with_labels ?key ?style ~config ~labels ~on_select ()
  in
  { view with
    kind = Tab_bar
  ; choice =
      Some
        { config
        ; appearance = None
        ; tab_appearance = appearance
        ; tab_viewport = viewport
        ; tab_motion = motion
        ; tab_trailing = false
        ; choice_menu = false
        ; tab_content = None
        ; on_select
        }
  }
;;

module Tab_content = struct
  type 'action view = 'action t

  module Label = struct
    type 'action t =
      | Default
      | Custom of 'action view
      | Hidden
  end

  type 'action t =
    { prefix : 'action view option
    ; label : 'action Label.t
    ; suffix : 'action view option
    }

  let create ?prefix ?(label = Label.Default) ?suffix () =
    let%map.Or_error () =
      match label with
      | Default | Hidden -> Ok ()
      | Custom view -> validate_control_labels [ view ]
    in
    { prefix; label; suffix }
  ;;
end

let tab_bar_with_content
      ?key
      ?style
      ?appearance
      ?max_width
      ?viewport
      ?motion
      ~config
      ~content
      ~on_select
      ()
  =
  let open Or_error.Let_syntax in
  let%bind () =
    match max_width with
    | Some width when (not (Float.is_finite width)) || Float.(width < 1. || width > 1e6)
      -> Or_error.error_string "tab maximum width must be finite and between 1 and 1e6"
    | None | Some _ -> Ok ()
  in
  let%bind content =
    Map.of_alist_or_error
      (module String)
      (List.map content ~f:(fun (id, part) -> Choice.Id.to_string id, part))
  in
  let options = Choice.Config.options config |> Choice.Collection.to_list in
  let ids =
    String.Set.of_list
      (List.map options ~f:(fun item -> Choice.Id.to_string (Choice.id item)))
  in
  let%bind () =
    match List.find (Map.keys content) ~f:(fun id -> not (Set.mem ids id)) with
    | None -> Ok ()
    | Some id -> Or_error.errorf "unknown tab content ID: %s" id
  in
  let%bind () =
    if
      List.exists options ~f:(fun item ->
        String.is_empty (String.strip (Choice.label item)))
    then Or_error.error_string "tab content requires meaningful configured names"
    else Ok ()
  in
  let slot name children =
    (* Structural slots have no presentation, including an empty base-style block. *)
    { (container ~key:(Key.of_string_exn name) [] children) with style = Style.empty }
  in
  let labels, children =
    List.map options ~f:(fun item ->
      let id = Choice.Id.to_string (Choice.id item) in
      let { Tab_content.prefix; label; suffix } =
        Option.value
          (Map.find content id)
          ~default:{ Tab_content.prefix = None; label = Default; suffix = None }
      in
      let mode, label =
        match label with
        | Default -> Gpuio_protocol.Wire.Tab_content.Label.Default, []
        | Hidden -> Hidden, []
        | Custom view -> Custom, [ view ]
      in
      ( mode
      , slot
          id
          [ slot "prefix" (Option.to_list prefix)
          ; slot "label" label
          ; slot "suffix" (Option.to_list suffix)
          ] ))
    |> List.unzip
  in
  let rec bounded count = function
    | [] -> Ok ()
    | (view, depth) :: rest ->
      if count >= 4096 || depth > 128
      then Or_error.error_string "tab content exceeds 4096 nodes or 128 levels"
      else
        bounded
          (count + 1)
          (List.rev_append
             (List.map view.children ~f:(fun child -> child, depth + 1))
             rest)
  in
  let%map () = bounded 0 (List.map children ~f:(fun view -> view, 1)) in
  { (tab_bar ?key ?style ?appearance ?viewport ?motion ~config ~on_select ()) with
    children
  ; choice =
      Some
        { config
        ; appearance = None
        ; tab_appearance = appearance
        ; tab_viewport = viewport
        ; tab_motion = motion
        ; tab_trailing = false
        ; choice_menu = false
        ; tab_content = Some { max_width; labels }
        ; on_select
        }
  }
;;

let select
      ?key
      ?(style = Style.empty)
      ?(appearance = Choice.Appearance.default)
      ~config
      ~on_select
      ()
  =
  { (radio_group ?key ~style ~config ~on_select ()) with
    kind = Select
  ; choice =
      Some
        { config
        ; appearance = Some appearance
        ; tab_appearance = None
        ; tab_viewport = None
        ; tab_motion = None
        ; tab_trailing = false
        ; choice_menu = false
        ; tab_content = None
        ; on_select
        }
  }
;;

let tab_bar_frame ?key ?style ?menu ?prefix ?suffix ?trailing tabs =
  let open Or_error.Let_syntax in
  let%bind choice =
    match tabs.kind, tabs.choice with
    | Tab_bar, Some choice when not choice.tab_trailing -> Ok choice
    | _ -> Or_error.error_string "tab frame requires a direct, unframed tab bar"
  in
  let structural role children =
    { (container [] children) with
      style = Style.empty
    ; structural_key = Some ("tab-frame", role)
    }
  in
  let options = Choice.Config.options choice.config |> Choice.Collection.to_list in
  let children =
    if List.is_empty tabs.children
    then
      List.map options ~f:(fun item ->
        { (container
             ~key:(Key.of_string_exn (Choice.Id.to_string (Choice.id item)))
             []
             [])
          with
          style = Style.empty
        })
    else tabs.children
  in
  let trailing =
    match trailing, Option.is_some suffix || Option.is_some menu with
    | Some view, _ -> [ view ]
    | None, true -> [ container [ Width (Length.px_exn 12.); Shrink 0. ] [] ]
    | None, false -> []
  in
  let viewport =
    { tabs with
      structural_key = Some ("tab-frame", "viewport")
    ; style =
        Style.merge
          [ Style.create_exn [ Grow 1.; Shrink 1.; Min_width (Length.px_exn 0.) ]
          ; tabs.style
          ]
    ; choice =
        Some
          { choice with
            tab_trailing = true
          ; tab_viewport =
              Some (Option.value choice.tab_viewport ~default:Tab_bar.Viewport.default)
          }
    ; children = children @ [ structural "trailing" trailing ]
    }
  in
  let fixed role content =
    let view = structural role (Option.to_list content) in
    { view with
      style =
        Style.create_exn
          [ Display (if Option.is_some content then Flex else Hidden)
          ; Direction Row
          ; Shrink 0.
          ]
    }
  in
  let%bind menu =
    Option.value_map menu ~default:(Ok None) ~f:(fun menu ->
      let label, style, appearance = Tab_bar.Expert.menu menu in
      let%bind config =
        Choice.Config.create
          ~label
          ~options:(Choice.Config.options choice.config)
          ~selected:(Choice.Config.selected choice.config)
          ~disabled:(Choice.Config.is_disabled choice.config)
          ()
      in
      let style =
        Style.merge
          [ Style.create_exn
              [ Width (Length.px_exn 28.)
              ; Height (Length.px_exn 28.)
              ; Padding (Length.px_exn 4.)
              ; Shrink 0.
              ]
          ; style
          ]
      in
      let icons =
        Tab_bar.Expert.menu_icons menu
        |> List.map ~f:(fun (id, decoration) -> Choice.Id.to_string id, decoration)
        |> String.Map.of_alist_exn
      in
      let ids =
        String.Set.of_list
          (List.map options ~f:(fun item -> Choice.Id.to_string (Choice.id item)))
      in
      let%bind () =
        match List.find (Map.keys icons) ~f:(fun key -> not (Set.mem ids key)) with
        | None -> Ok ()
        | Some key -> Or_error.errorf "unknown tab menu icon ID: %s" key
      in
      let children =
        if Map.is_empty icons
        then []
        else
          List.map options ~f:(fun item ->
            let key = Choice.Id.to_string (Choice.id item) in
            let child =
              Option.map (Map.find icons key) ~f:(fun decoration ->
                let config, style = Icon.Expert.decoration decoration in
                icon ~style config)
            in
            { (container ~key:(Key.of_string_exn key) [] (Option.to_list child)) with
              style = Style.empty
            })
      in
      let%map () = validate_control_labels children in
      let view = select ~style ~appearance ~config ~on_select:choice.on_select () in
      Some
        { view with
          children
        ; choice =
            Option.map view.choice ~f:(fun choice -> { choice with choice_menu = true })
        })
  in
  let view =
    row
      ?key:(Option.first_some key tabs.key)
      ?style
      ([ fixed "prefix" prefix; viewport ]
       @ (if Option.is_some menu then [ fixed "menu" menu ] else [])
       @ [ fixed "suffix" suffix ])
  in
  let rec bounded count = function
    | [] -> Ok ()
    | (view, depth) :: rest ->
      if count >= 4096 || depth > 128
      then Or_error.error_string "tab frame exceeds 4096 nodes or 128 levels"
      else
        bounded
          (count + 1)
          (List.rev_append
             (List.map view.children ~f:(fun child -> child, depth + 1))
             rest)
  in
  let%map () = bounded 0 [ view, 1 ] in
  view
;;

let combobox
      ?(style = Style.empty)
      ?(appearance = Choice.Appearance.default)
      ?(initial_text = "")
      ~controller
      ~config
      ~on_event
      ()
  =
  let%map.Or_error () = Text_input.validate_text ~mode:Single_line initial_text in
  { key = Some controller
  ; structural_key = None
  ; kind = Combobox
  ; text = initial_text
  ; text_content = None
  ; text_shimmer = None
  ; scrollbar = None
  ; window_region = None
  ; link = None
  ; style
  ; on_hover = None
  ; on_click = None
  ; editor = None
  ; split_button = None
  ; button_presentation = None
  ; tab_order = None
  ; control_appearance = None
  ; control = None
  ; choice = None
  ; combobox = Some { controller; config; appearance; on_event }
  ; choice_picker = None
  ; popover = false
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; command_binding_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; progress_presentation = None
  ; loading = None
  ; spinner = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; reveal = None
  ; navigation_stack = None
  ; carousel = None
  ; carousel_track_motion = None
  ; carousel_track = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; split_group = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_header = None
  ; table_header_style = None
  ; table_row_style = None
  ; table_cell = None
  ; children = []
  }
;;

let choice_picker ?key ?style ~on_event description =
  let open Or_error.Let_syntax in
  let module D = Choice_picker.Description in
  let config = D.config description in
  let wrap role identity content =
    { (text "") with
      kind = Container
    ; structural_key = Some ("choice-picker:" ^ role, identity)
    ; children = [ content ]
    }
  in
  let passive =
    Option.to_list (D.trigger description)
    @ Option.to_list (D.empty description)
    @ List.map (D.groups description) ~f:snd
    @ List.map (D.options description) ~f:(fun (_, item) ->
      Choice_picker.Option_content.content item)
  in
  let%bind () =
    validate_passive_children
      ~allow_progress:true
      ~context:"picker passive content"
      ~validate_style:Style.Expert.validate_control_label
      passive
  in
  let%bind query =
    match D.query description with
    | None -> Ok []
    | Some query ->
      let%map editor_config =
        Text_input.Config.create
          ~mode:Single_line
          ~label:(Choice_picker.Config.label config)
          ~placeholder:(Choice_picker.Config.search_placeholder config)
          ~disabled:(Choice_picker.Config.is_disabled config)
          ~submit_on_enter:false
          ~auto_focus:false
          ~min_rows:1
          ~max_rows:1
          ()
      in
      let input =
        { (text
             ~key:(Choice_picker.Query.controller query)
             (Choice_picker.Query.initial_text query))
          with
          kind = Input
        ; editor =
            Some
              { controller = Choice_picker.Query.controller query
              ; config = editor_config
              ; frame = None
              ; on_event = Picker_query
              }
        }
      in
      [ wrap "query" "" input ]
  in
  let children =
    List.map (Option.to_list (D.trigger description)) ~f:(wrap "trigger" "")
    @ query
    @ List.map (Option.to_list (D.empty description)) ~f:(wrap "empty" "")
    @ List.map (Option.to_list (D.footer description)) ~f:(wrap "footer" "")
    @ List.map (D.groups description) ~f:(fun (id, content) ->
      wrap "group" (Choice_picker.Group.Id.to_string id) content)
    @ List.map (D.options description) ~f:(fun (id, item) ->
      wrap "option" (Choice.Id.to_string id) (Choice_picker.Option_content.content item))
  in
  let rec budget count = function
    | [] -> Ok ()
    | (node, depth) :: rest ->
      if count >= 4096 || depth > 128
      then Or_error.error_string "picker content exceeds 4096 nodes or 128 levels"
      else if List.length node.children > 4096 - count - List.length rest
      then Or_error.error_string "picker content exceeds 4096 nodes"
      else
        budget
          (count + 1)
          (List.rev_append
             (List.map node.children ~f:(fun child -> child, depth + 1))
             rest)
  in
  let%map () = budget 0 (List.map children ~f:(fun child -> child, 1)) in
  { (text ?key ?style "") with
    kind = Choice_picker
  ; choice_picker = Some (description, on_event)
  ; children
  }
;;

let command_palette
      ?key
      ?(style = Style.empty)
      ?(appearance =
        Choice.Appearance.create
          ~popup_width:560.
          ~max_visible_rows:8
          ~empty_label:"No matching commands"
          ()
        |> Or_error.ok_exn)
      ~config
      ~on_dismiss
      ()
  =
  { (text ?key ~style "") with
    kind = Command_palette
  ; palette = Some { config; appearance; on_dismiss }
  }
;;

let slider ?(style = Style.empty) ?appearance ~controller ~config ~initial ~on_event () =
  { (text ~key:controller ~style "") with
    kind = Slider
  ; slider = Some { controller; config; appearance; initial; on_event }
  }
;;

let number_input
      ?(style = Style.empty)
      ~controller
      ~config
      ~initial
      ?initial_draft
      ~on_event
      ()
  =
  { (text ~key:controller ~style "") with
    kind = Number_input
  ; number_input =
      Some { controller; config; appearance = None; initial; initial_draft; on_event }
  }
;;

let otp_input ?(style = Style.empty) ?appearance ~controller ~config ~initial ~on_event ()
  =
  { (text ~key:controller ~style "") with
    kind = Otp_input
  ; otp_input = Some { controller; appearance; config; initial; on_event }
  }
;;

let color_input
      ?(style = Style.empty)
      ?appearance
      ~controller
      ~config
      ~initial
      ~on_event
      ()
  =
  { (text ~key:controller ~style "") with
    kind = Color_input
  ; color_input = Some { controller; config; appearance; initial; on_event }
  }
;;

let calendar
      ?(style = Style.empty)
      ?appearance
      ?content
      ?on_viewport_change
      ~controller
      ~config
      ~initial
      ~initial_month
      ~on_event
      ()
  =
  { (text ~key:controller ~style "") with
    kind = Calendar
  ; calendar =
      Some
        { controller
        ; config
        ; initial
        ; initial_month
        ; appearance
        ; on_viewport_change
        ; on_event
        ; content = Option.map content ~f:(fun c -> c.Calendar_content.metadata)
        }
  ; children =
      Option.value_map content ~default:[] ~f:(fun c -> c.Calendar_content.children)
  }
;;

let rating ?key ?(style = Style.empty) ?appearance ~config ~on_request () =
  { (text ?key ~style "") with
    kind = Rating
  ; rating = Some { config; appearance; on_request }
  }
;;

let avatar ?key ?(style = Style.empty) ?on_change config =
  { (text ?key ~style "") with
    kind = Avatar
  ; avatar = Some config
  ; image =
      Option.map (Avatar.Expert.image config) ~f:(fun config -> { config; on_change })
  }
;;

let avatar_with_fallback ?key ?style ?on_change config ~fallback =
  let%map.Or_error () =
    validate_passive_children
      ~context:"avatar fallback"
      ~validate_style:Style.Expert.validate_avatar_fallback
      [ fallback ]
  in
  { (avatar ?key ?style ?on_change config) with children = [ fallback ] }
;;

let spinner ?key ?(style = Style.empty) ?on_icon_change ~config () =
  { (text ?key ~style "") with
    kind = Loading
  ; spinner = Some config
  ; loading = Some (Spinner.Expert.loading config)
  ; image =
      Option.map (Spinner.Expert.image config) ~f:(fun config ->
        { config; on_change = on_icon_change })
  }
;;

let number_frame
      ?(appearance = Number_input.Appearance.default)
      ?leading
      ?trailing
      ?decrement
      ?increment
      child
  =
  match child.kind, child.number_input with
  | Number_input, Some input ->
    let open Or_error.Let_syntax in
    let%map () = validate_control_labels (List.filter_opt [ decrement; increment ]) in
    let slot role content =
      { (text "") with
        kind = Container
      ; structural_key = Some ("number-frame:" ^ role, "")
      ; children = Option.to_list content
      }
    in
    { child with
      number_input = Some { input with appearance = Some appearance }
    ; children =
        [ slot "leading" leading
        ; slot "trailing" trailing
        ; slot "decrement" decrement
        ; slot "increment" increment
        ]
    }
  | _ -> Or_error.error_string "number frame requires a direct number_input view"
;;

let input_frame ?(config = Input_frame.default) ?leading ?trailing ?on_reveal child =
  let open Or_error.Let_syntax in
  match child.kind, child.editor with
  | (Input | Textarea), Some ({ on_event = Editor_events _; _ } as editor) ->
    let wire = Input_frame.Expert.to_wire config in
    let%bind () =
      if Kind.equal child.kind Textarea && Option.is_some wire.clear_label
      then Or_error.error_string "input frame clear requires a single-line input"
      else Ok ()
    in
    let%bind reveal =
      match on_reveal, Text_input.Config.privacy editor.config with
      | None, _ -> Ok None
      | Some _, Plain ->
        Or_error.error_string "input frame reveal requires a password input"
      | Some on_click, Password display ->
        let revealed = Text_input.Password_display.equal display Revealed in
        let label = Input_frame.Expert.reveal_label config ~revealed in
        Ok
          (Some (button ~config:(Button.Config.create ~focus:Preserve ()) ~on_click label))
    in
    let%map loading =
      if Input_frame.Expert.is_loading config
      then (
        let%map config =
          Spinner.Config.create ~label:(Input_frame.Expert.loading_label config) ()
        in
        Some (spinner ~config ()))
      else Ok None
    in
    let slot role content =
      { (text "") with
        kind = Container
      ; structural_key = Some ("input-frame:" ^ role, "")
      ; children = Option.to_list content
      }
    in
    { child with
      editor = Some { editor with frame = Some config }
    ; children =
        [ slot "leading" leading
        ; slot "loading" loading
        ; slot "reveal" reveal
        ; slot "trailing" trailing
        ]
    }
  | _ -> Or_error.error_string "input frame requires a direct text_input view"
;;

let loading ?key ?(style = Style.empty) ~config () =
  { (text ?key ~style "") with kind = Loading; loading = Some config }
;;

let progress ?key ?(style = Style.empty) ?transition ~config () =
  { (text ?key ~style "") with
    kind = Progress
  ; progress = Some config
  ; progress_presentation =
      Option.map transition ~f:(fun transition ->
        Progress.Expert.presentation_to_wire config ~shape:Linear ~transition)
  }
;;

let progress_circle ?key ?(style = Style.empty) ?transition ~config children =
  let transition =
    Option.value_or_thunk transition ~default:(fun () ->
      Progress.Transition.tween (Time_ns.Span.of_ms 200.) |> Or_error.ok_exn)
  in
  { (text ?key ~style "") with
    kind = Progress
  ; progress = Some config
  ; progress_presentation =
      Some (Progress.Expert.presentation_to_wire config ~shape:Circle ~transition)
  ; children
  }
;;

let drag_source ?key ?(style = Style.empty) ~config ~on_event children =
  { (column ?key ~style children) with
    kind = Drag_source
  ; drag_source = Some { config; on_event }
  }
;;

let drop_target ?key ?(style = Style.empty) ~config ~on_event children =
  { (column ?key ~style children) with
    kind = Drop_target
  ; drop_target = Some { config; on_event }
  }
;;

let command_binding_scope ?key ?(style = Style.empty) ~config ~on_update children =
  { (column ?key ~style children) with
    command_binding_scope = Some { config; on_update }
  }
;;

let highlight_scope ?key ?(style = Style.empty) ~config ?on_update children =
  { (column ?key ~style children) with
    kind = Highlight_scope
  ; highlight_scope = Some { config; on_update }
  }
;;

let input_region ?key ?(style = Style.empty) ~config ~on_event children =
  { (column ?key ~style children) with
    kind = Input_region
  ; input_region = Some { config; on_event }
  }
;;

let pointer_area ?key ?(style = Style.empty) ~config ~on_event children =
  { (column ?key ~style children) with
    kind = Pointer_area
  ; pointer = Some { config; on_event }
  }
;;

let toast ~key ?(style = Style.empty) ~config ~on_dismiss children =
  Toast_item
    { (column ~key ~style children) with
      kind = Toast
    ; notification = Some { config; on_dismiss }
    }
;;

let toast_stack ?key ?(style = Style.empty) ?(config = Toast.Stack.default) items =
  let children = List.map items ~f:(fun (Toast_item view) -> view) in
  let keys =
    List.map children ~f:(fun view -> Key.to_string (Option.value_exn view.key))
  in
  if List.length children > 32 || Set.length (String.Set.of_list keys) <> List.length keys
  then Or_error.error_string "toast stack requires at most 32 uniquely keyed items"
  else
    Ok
      { (column ?key ~style children) with kind = Toast_stack; toast_stack = Some config }
;;

module Expert = struct
  type nonrec 'action table = 'action table =
    { source_key : Key.t option
    ; config : Table.Config.t
    ; query_generation : int64
    ; commands : Key.t Table.Command.t list
    ; on_input : Key.t Table.Request.t -> 'action
    ; on_column_viewport : (Table.Column_viewport.t -> 'action) option
    }

  type nonrec 'action virtual_list = 'action virtual_list =
    { config : Virtual_list.Config.t
    ; order : Virtual_list.Order.t
    ; managed : bool
    ; invalidated : Key.t list
    ; invalidation_revision : int64
    ; scroll : Virtual_list.Scroll_request.t option
    ; on_viewport : (Virtual_list.Viewport.t -> 'action) option
    ; on_retain : (Key.t list -> 'action) option
    ; on_tree_input : (Key.t Tree_input.t -> 'action) option
    ; list_input : (List_input.Config.t * (Key.t List_input.t -> 'action)) option
    ; tree_moves : bool
    ; table : 'action table option
    }

  let managed_virtual_list
        ?key
        ?style
        ?scroll
        ?(invalidated = [])
        ?(invalidation_revision = 0L)
        ~config
        ~order
        ~on_viewport
        ~on_retain
        ?on_tree_input
        ?(tree_moves = false)
        rows
    =
    make_virtual_list
      ?key
      ?style
      ~config
      ~order
      ~managed:true
      ~invalidated
      ~invalidation_revision
      ~scroll
      ~on_viewport:(Some on_viewport)
      ~on_retain:(Some on_retain)
      ~on_tree_input
      ~tree_moves
      rows
  ;;

  let table_text metadata =
    { (text (Table.Cell.copy_text metadata)) with table_cell = Some metadata }
  ;;

  let managed_table
        ?key
        ?source_key
        ?style
        ?(headers = [])
        ?header_presentation
        ?(row_presentations = [])
        ?(commands = [])
        ?on_column_viewport
        ~config
        ~query_generation
        ~order
        ~on_viewport
        ~on_retain
        ~on_input
        rows
    =
    let open Or_error.Let_syntax in
    let%bind (_ : Gpuio_protocol.Table_wire.Config.t) =
      Table.Expert.to_wire config ~schema_revision:1L ~query_generation
    in
    let%bind () =
      Table_header.validate_all headers ~columns:(Table.Config.columns config)
    in
    let%bind row_presentations =
      Map.of_alist_or_error
        (module String)
        (List.map row_presentations ~f:(fun (key, style) -> Key.to_string key, style))
    in
    let active_keys =
      String.Set.of_list (List.map rows ~f:(fun (key, _) -> Key.to_string key))
    in
    let%bind () =
      if Map.for_alli row_presentations ~f:(fun ~key ~data:_ -> Set.mem active_keys key)
      then Ok ()
      else Or_error.error_string "row presentation must belong to an active table row"
    in
    let columns =
      Table_column.Collection.to_list (Table.Config.columns config)
      |> List.map ~f:Table_column.id
    in
    if
      List.length commands > 64
      || not
           (List.for_all rows ~f:(fun (_, cells) ->
              List.equal
                Table_column.Id.equal
                columns
                (List.map cells ~f:(fun (cell, _) -> Table.Cell.column cell))))
    then Or_error.error_string "table rows must match schema columns; at most 64 commands"
    else (
      let%map root =
        make_virtual_list
          ?key
          ?style
          ~config:(Table.Expert.list_config config)
          ~order
          ~managed:true
          ~invalidated:[]
          ~invalidation_revision:0L
          ~scroll:None
          ~on_viewport:(Some on_viewport)
          ~on_retain:(Some on_retain)
          ~on_tree_input:None
          ~tree_moves:false
          (List.map rows ~f:(fun (key, _) -> key, column []))
      in
      let children =
        List.map rows ~f:(fun (key, cells) ->
          let row =
            column
              ~key
              (List.map cells ~f:(fun (metadata, child) ->
                 let key =
                   Key.of_string_exn
                     (Table_column.Id.to_string (Table.Cell.column metadata))
                 in
                 match child.kind, child.table_cell with
                 | Text, Some compact when Table.Cell.equal compact metadata ->
                   { child with key = Some key }
                 | _ -> { (column ~key [ child ]) with table_cell = Some metadata }))
          in
          { row with table_row_style = Map.find row_presentations (Key.to_string key) })
      in
      let headers =
        List.map headers ~f:(fun header ->
          { (container ~key:(Table_header.key header) [] [ Table_header.content header ]) with
            style = Style.empty
          ; structural_key = Some ("table-header", Key.to_string (Table_header.key header))
          ; table_header = Some (Table_header.target header)
          })
      in
      { root with
        children = headers @ children
      ; table_header_style = header_presentation
      ; virtual_list =
          Option.map root.virtual_list ~f:(fun list ->
            { list with
              table =
                Some
                  { source_key
                  ; config
                  ; query_generation
                  ; commands
                  ; on_input
                  ; on_column_viewport
                  }
            })
      })
  ;;

  type nonrec 'action container_query = 'action container_query =
    { config : Container_query.Config.t
    ; on_select : (Container_query.Selection.t -> 'action) option
    }

  type nonrec 'action animation_program = 'action animation_program =
    { config : Animation.Program.t
    ; on_event : (Animation.Program.Event.t -> 'action) option
    }

  type nonrec 'action animation = 'action animation =
    { config : Animation.Config.t
    ; on_event : (Animation.Event.t -> 'action) option
    }

  type nonrec 'action extension = 'action extension =
    { config : Gpuio_protocol.Extension_wire.Config.t
    ; on_event : Gpuio_protocol.Extension_wire.Signal.t -> 'action
    }

  type nonrec 'action split_group = 'action split_group =
    { config : Split_group.Config.t
    ; appearance : Split_group.Appearance.t
    ; on_resize : (Split_group.Snapshot.t -> 'action) option
    }

  type nonrec 'action split_pane = 'action split_pane =
    { config : Split_pane.Config.t
    ; on_resize : (Split_pane.Snapshot.t -> 'action) option
    }

  type nonrec 'action canvas = 'action canvas =
    { config : Canvas.Config.t
    ; on_event : (Canvas.Event.t -> 'action) option
    }

  type nonrec 'action chart = 'action chart =
    { config : Chart.Config.t
    ; on_event : (Chart.Event.t -> 'action) option
    }

  type nonrec 'action document = 'action document =
    { config : Document.Config.t
    ; on_navigate : (Document.Navigation.t -> 'action) option
    ; on_diff : (Document.Diff.Event.t -> 'action) option
    ; on_preview : (Document.Preview.Event.t -> 'action) option
    ; on_action : (Document.Actions.Event.t -> 'action) option
    ; inherit_profile : bool
    ; profile :
        (Gpuio_protocol.Document_profile_wire.Instance.t
        * (Gpuio_protocol.Document_profile_wire.Event.t -> 'action option))
          option
    }

  type nonrec 'action slider = 'action slider =
    { controller : Key.t
    ; config : Slider.Config.t
    ; appearance : Slider.Appearance.t option
    ; initial : Slider.Value.t
    ; on_event : Slider.Event.t -> 'action
    }

  type nonrec 'action number_input = 'action number_input =
    { controller : Key.t
    ; config : Number_input.Config.t
    ; appearance : Number_input.Appearance.t option
    ; initial : Number_input.Value.t
    ; initial_draft : Number_input.Draft.t option
    ; on_event : Number_input.Event.t -> 'action
    }

  type nonrec 'action otp_input = 'action otp_input =
    { controller : Key.t
    ; appearance : Otp_input.Appearance.t option
    ; config : Otp_input.Config.t
    ; initial : Otp_input.Value.t
    ; on_event : Otp_input.Event.t -> 'action
    }

  type nonrec 'action color_input = 'action color_input =
    { controller : Key.t
    ; config : Color_input.Config.t
    ; appearance : Color_input.Appearance.t option
    ; initial : Color_value.Value.t
    ; on_event : Color_input.Event.t -> 'action
    }

  type nonrec 'action calendar = 'action calendar =
    { controller : Key.t
    ; config : Calendar.Config.t
    ; initial : Calendar.Selection.t
    ; initial_month : Calendar.Month.t
    ; appearance : Calendar.Appearance.t option
    ; content : Gpuio_protocol.Calendar_content_wire.t option
    ; on_event : Calendar.Event.t -> 'action
    ; on_viewport_change : (Calendar.Viewport.t -> 'action) option
    }

  type nonrec 'action rating = 'action rating =
    { config : Rating.Config.t
    ; appearance : Rating.Appearance.t option
    ; on_request : Rating.Request.t -> 'action
    }

  type nonrec 'action image = 'action image =
    { config : Image.Config.t
    ; on_change : (Image.State.t -> 'action) option
    }

  type nonrec 'action drag_source = 'action drag_source =
    { config : Drag_and_drop.Source.t
    ; on_event : Drag_and_drop.Source_event.t -> 'action
    }

  type nonrec 'action drop_target = 'action drop_target =
    { config : Drag_and_drop.Target.t
    ; on_event : Drag_and_drop.Target_event.t -> 'action
    }

  type nonrec 'action command_binding_scope = 'action command_binding_scope =
    { config : Command_binding.Config.t
    ; on_update : Command_binding.Observation.t -> 'action
    }

  type nonrec 'action highlight_scope = 'action highlight_scope =
    { config : Highlight.Config.t
    ; on_update : (Highlight.Observation.t -> 'action) option
    }

  type nonrec 'action input_region = 'action input_region =
    { config : Input_region.Config.t
    ; on_event : Input_region.Event.t -> 'action
    }

  type nonrec 'action pointer = 'action pointer =
    { config : Pointer.Config.t
    ; on_event : Pointer.Event.t -> 'action
    }

  type nonrec 'action notification = 'action notification =
    { config : Toast.Config.t
    ; on_dismiss : Toast.Dismissal.t -> 'action
    }

  module Kind = Kind
  module Control = Control

  type nonrec 'action tooltip = 'action tooltip =
    { config : Tooltip.Config.t
    ; on_open_change : (bool -> 'action) option
    }

  type nonrec 'action overlay = 'action overlay =
    { kind : Gpuio_protocol.Wire.Overlay_kind.t
    ; config : Overlay.Config.t
    ; backdrop : Color.t option
    ; motion : Overlay.Motion.t
    ; sheet_insets : Sheet.Insets.t option
    ; on_dismiss : Overlay.Dismissal.t -> 'action
    }

  type nonrec 'action combobox = 'action combobox =
    { controller : Key.t
    ; config : Combobox.Config.t
    ; appearance : Choice.Appearance.t
    ; on_event : Combobox.Event.t -> 'action
    }

  type nonrec 'action choice = 'action choice =
    { config : Choice.Config.t
    ; appearance : Choice.Appearance.t option
    ; tab_appearance : Tab_bar.Appearance.t option
    ; tab_content : Gpuio_protocol.Wire.Tab_content.t option
    ; tab_viewport : Tab_bar.Viewport.t option
    ; tab_motion : Tab_bar.Motion.t option
    ; tab_trailing : bool
    ; choice_menu : bool
    ; on_select : Choice.Id.t -> 'action
    }

  type nonrec 'action editor_callback = 'action editor_callback =
    | Editor_events of (Text_input.Event.t -> 'action)
    | Picker_query

  type nonrec 'action editor = 'action editor =
    { controller : Key.t
    ; config : Text_input.Config.t
    ; frame : Input_frame.t option
    ; on_event : 'action editor_callback
    }

  type nonrec 'action palette = 'action palette =
    { config : Command_palette.Config.t
    ; appearance : Command_palette.Appearance.t
    ; on_dismiss : Command_palette.Dismissal.t -> 'action
    }

  type nonrec 'action menu = 'action menu =
    { presentation : Menu.Expert.presentation
    ; menus : Menu.t list
    ; appearance : Menu.Appearance.t
    ; placement : Placement.t option
    ; on_open_change : (bool -> 'action) option
    }

  type 'action description = 'action t =
    { key : Key.t option
    ; structural_key : (string * string) option
    ; kind : Kind.t
    ; text : string
    ; text_content : Text_content.t option
    ; text_shimmer : Text_shimmer.Config.t option
    ; scrollbar : Scrollbar.t option
    ; window_region : Window_region.t option
    ; link : Link.Config.t option
    ; style : Style.t
    ; on_click : (unit -> 'action) option
    ; on_hover : (bool -> 'action) option
    ; editor : 'action editor option
    ; control : Control.t option
    ; split_button :
        (Split_button.Appearance.t * Gpuio_protocol.Wire.Split_button.Parts.t) option
    ; button_presentation : Button.Expert.Presentation.t option
    ; tab_order : Tab_order.t option
    ; control_appearance : Control_appearance.t option
    ; choice : 'action choice option
    ; combobox : 'action combobox option
    ; choice_picker :
        ('action t Choice_picker.Description.t * (Choice_picker.Event.t -> 'action))
          option
    ; popover : bool
    ; overlay : 'action overlay option
    ; tooltip : 'action tooltip option
    ; commands : 'action Ui_command.Registry.t option
    ; command_ref : Ui_command.Id.t option
    ; drag_source : 'action drag_source option
    ; drop_target : 'action drop_target option
    ; pointer : 'action pointer option
    ; input_region : 'action input_region option
    ; highlight_scope : 'action highlight_scope option
    ; command_binding_scope : 'action command_binding_scope option
    ; notification : 'action notification option
    ; toast_stack : Toast.Stack.t option
    ; progress : Progress.Config.t option
    ; progress_presentation : Gpuio_protocol.Progress_wire.Presentation.t option
    ; loading : Loading.Config.t option
    ; spinner : Spinner.Config.t option
    ; avatar : Avatar.Config.t option
    ; rating : 'action rating option
    ; slider : 'action slider option
    ; number_input : 'action number_input option
    ; otp_input : 'action otp_input option
    ; color_input : 'action color_input option
    ; calendar : 'action calendar option
    ; animation : 'action animation option
    ; animation_program : 'action animation_program option
    ; reveal : Gpuio_protocol.Reveal_wire.t option
    ; navigation_stack : Gpuio_protocol.Navigation_stack_wire.Config.t option
    ; carousel :
        (Gpuio_protocol.Carousel_wire.Config.t * (Carousel.Request.t -> 'action)) option
    ; carousel_track_motion : Gpuio_protocol.Carousel_track_wire.Motion.t option
    ; carousel_track :
        (Gpuio_protocol.Carousel_track_wire.Config.t
        * (Carousel_track.Request.t -> 'action))
          option
    ; container_query : 'action container_query option
    ; accessibility : Accessibility.t option
    ; image : 'action image option
    ; extension : 'action extension option
    ; split_pane : 'action split_pane option
    ; split_group : 'action split_group option
    ; document : 'action document option
    ; canvas : 'action canvas option
    ; chart : 'action chart option
    ; palette : 'action palette option
    ; menu : 'action menu option
    ; focus_scope : Focus_scope.t option
    ; virtual_list : 'action virtual_list option
    ; table_header : Table_header.Target.t option
    ; table_header_style : Table_presentation.Header.t option
    ; table_row_style : Table_presentation.Row.t option
    ; table_cell : Table.Cell.t option
    ; children : 'action t list
    }

  let describe t = t
end
