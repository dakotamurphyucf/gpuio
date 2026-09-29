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
    | Chart_view
    | Input_region
    | Highlight_scope
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
  ;;
end

type 'action editor =
  { controller : Key.t
  ; config : Text_input.Config.t
  ; on_event : Text_input.Event.t -> 'action
  }

type 'action slider =
  { controller : Key.t
  ; config : Slider.Config.t
  ; initial : Slider.Value.t
  ; on_event : Slider.Event.t -> 'action
  }

type 'action number_input =
  { controller : Key.t
  ; config : Number_input.Config.t
  ; initial : Number_input.Value.t
  ; on_event : Number_input.Event.t -> 'action
  }

type 'action otp_input =
  { controller : Key.t
  ; config : Otp_input.Config.t
  ; initial : Otp_input.Value.t
  ; on_event : Otp_input.Event.t -> 'action
  }

type 'action color_input =
  { controller : Key.t
  ; config : Color_input.Config.t
  ; initial : Color_value.Value.t
  ; on_event : Color_input.Event.t -> 'action
  }

type 'action calendar =
  { controller : Key.t
  ; config : Calendar.Config.t
  ; initial : Calendar.Selection.t
  ; initial_month : Calendar.Month.t
  ; on_event : Calendar.Event.t -> 'action
  }

type 'action rating =
  { config : Rating.Config.t
  ; on_request : Rating.Request.t -> 'action
  }

type 'action choice =
  { config : Choice.Config.t
  ; appearance : Choice.Appearance.t option
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
  ; on_dismiss : Overlay.Dismissal.t -> 'action
  }

type 'action tooltip =
  { config : Tooltip.Config.t
  ; on_open_change : (bool -> 'action) option
  }

type menu =
  { presentation : Menu.Expert.presentation
  ; menus : Menu.t list
  ; appearance : Menu.Appearance.t
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
  ; tree_moves : bool
  ; table : 'action table option
  }

type 'action t =
  { key : Key.t option
  ; kind : Kind.t
  ; text : string
  ; style : Style.t
  ; on_click : (unit -> 'action) option
  ; editor : 'action editor option
  ; control : Control.t option
  ; choice : 'action choice option
  ; combobox : 'action combobox option
  ; overlay : 'action overlay option
  ; tooltip : 'action tooltip option
  ; commands : 'action Ui_command.Registry.t option
  ; command_ref : Ui_command.Id.t option
  ; drag_source : 'action drag_source option
  ; drop_target : 'action drop_target option
  ; pointer : 'action pointer option
  ; input_region : 'action input_region option
  ; highlight_scope : 'action highlight_scope option
  ; notification : 'action notification option
  ; toast_stack : Toast.Stack.t option
  ; progress : Progress.Config.t option
  ; loading : Loading.Config.t option
  ; avatar : Avatar.Config.t option
  ; rating : 'action rating option
  ; slider : 'action slider option
  ; number_input : 'action number_input option
  ; otp_input : 'action otp_input option
  ; color_input : 'action color_input option
  ; calendar : 'action calendar option
  ; animation : 'action animation option
  ; animation_program : 'action animation_program option
  ; navigation_stack : Gpuio_protocol.Navigation_stack_wire.Config.t option
  ; carousel :
      (Gpuio_protocol.Carousel_wire.Config.t * (Carousel.Request.t -> 'action)) option
  ; container_query : 'action container_query option
  ; accessibility : Accessibility.t option
  ; image : 'action image option
  ; extension : 'action extension option
  ; split_pane : 'action split_pane option
  ; document : 'action document option
  ; canvas : 'action canvas option
  ; chart : 'action chart option
  ; palette : 'action palette option
  ; menu : menu option
  ; focus_scope : Focus_scope.t option
  ; virtual_list : 'action virtual_list option
  ; table_cell : Table.Cell.t option
  ; children : 'action t list
  }

type 'action toast = Toast_item of 'action t

let text ?key ?(style = Style.empty) text =
  { key
  ; kind = Text
  ; text
  ; style
  ; on_click = None
  ; editor = None
  ; choice = None
  ; combobox = None
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
  ; control = None
  ; children = []
  }
;;

let animate_program ?key ?(style = Style.empty) ?on_event config children =
  { (text ?key ~style "") with
    kind = Animation_program
  ; animation_program = Some { config; on_event }
  ; children
  }
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
        | Radio_group
        | Select
        | Rating
        | Slider
        | Number_input
        | Otp_input
        | Calendar
        | Color_input ) ) -> true
    | None, Some Link, (Button | Command_button) -> true
    | None, Some Navigation, Container -> true
    | None, Some (Tree _), Virtual_list -> true
    | None, Some (Tree_item _), Container -> true
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
        | Command_button
        | Input
        | Textarea
        | Combobox
        | Checkbox
        | Switch
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
    | Text | Button | Command_button -> true
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

let document ?key ?(style = Style.empty) ?on_navigate ?on_diff config =
  { (text ?key ~style "") with
    kind = Document_view
  ; document = Some { config; on_navigate; on_diff }
  }
;;

let icon ?key ?style ?on_change config =
  { (image ?key ?style ?on_change (Icon.Expert.image config)) with kind = Icon }
;;

let container ?key ?(style = Style.empty) defaults children =
  { key
  ; kind = Container
  ; text = ""
  ; style = Style.merge [ Style.create_exn defaults; style ]
  ; on_click = None
  ; editor = None
  ; choice = None
  ; combobox = None
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
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
  ; kind = Button
  ; text
  ; style = button_style style
  ; on_click = (if disabled then None else Some on_click)
  ; editor = None
  ; choice = None
  ; combobox = None
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
  ; control = Some (Button { disabled })
  ; children
  }
;;

let icon_button ?key ?style ?disabled ~label ~on_click icon =
  button ?key ?style ?disabled ~accessible_name:label ~leading_icon:icon ~on_click ""
;;

let toggle
      ?key
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
      ; Column_gap (Length.px_exn 8.)
      ; Padding (Length.px_exn 6.)
      ; Radius 4.
      ; Border_width 1.
      ; Border_color (Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> Or_error.ok_exn)
      ; Foreground (Color.token_exn "foreground")
      ]
    |> fun t -> Style.with_state_exn t Focused [ Border_color (Color.token_exn "accent") ]
  in
  { key
  ; kind
  ; text
  ; style = Style.merge [ defaults; style ]
  ; on_click = (if disabled then None else Some on_toggle)
  ; editor = None
  ; choice = None
  ; combobox = None
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
  ; control = Some control
  ; children = []
  }
;;

let checkbox ?key ?style ?accessible_name ?(disabled = false) ~state ~on_toggle text =
  toggle
    ?key
    ?style
    ?accessible_name
    ~disabled
    ~control:(Checkbox { state; disabled })
    ~kind:Checkbox
    ~on_toggle
    text
;;

let switch ?key ?style ?accessible_name ?(disabled = false) ~checked ~on_toggle text =
  toggle
    ?key
    ?style
    ?accessible_name
    ~disabled
    ~control:(Switch { checked; disabled })
    ~kind:Switch
    ~on_toggle
    text
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
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

let modal_overlay ?key ?style ~kind ~config ~on_dismiss content =
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
      overlay = Some { kind; config; on_dismiss }
    }
;;

let dialog ?key ?style ~config ~on_dismiss content =
  modal_overlay ?key ?style ~kind:Dialog ~config ~on_dismiss content
;;

let sheet ?key ?style ~config ~on_dismiss content =
  modal_overlay
    ?key
    ?style
    ~kind:(Sheet.Expert.kind config)
    ~config:(Sheet.Expert.overlay config)
    ~on_dismiss
    content
;;

let alert_dialog ?key ?style ~config ~on_dismiss content =
  modal_overlay
    ?key
    ?style
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
          overlay = Some { kind = Popover; config; on_dismiss }
        }
      in
      [ anchor; panel ]
  in
  container ?key [ Position Relative ] children
;;

let command_scope ?key ?style ~commands children =
  { (container ?key ?style [] children) with
    kind = Command_scope
  ; commands = Some commands
  }
;;

let command_button ?key ?style ?leading_icon ?trailing_icon ~command () =
  let children = icon_slots leading_icon trailing_icon in
  let style =
    button_style (icon_button_style (Option.value style ~default:Style.empty) children)
  in
  { (text ?key ~style "") with
    kind = Command_button
  ; on_click = None
  ; control = None
  ; command_ref = Some command
  ; children
  }
;;

let menu_button
      ?key
      ?(style = Style.empty)
      ?(appearance = Menu.Appearance.default)
      ~menu
      ()
  =
  { (text ?key ~style:(button_style style) "") with
    kind = Menu
  ; menu = Some { presentation = Button; menus = [ menu ]; appearance }
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
  ; menu = Some { presentation = Context; menus = [ menu ]; appearance }
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
  let%map.Or_error () = Menu.Expert.validate_collection menus in
  { (text ?key ~style "") with
    kind = Menu
  ; menu =
      Some { presentation = (if platform then Platform_bar else Bar); menus; appearance }
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

let panel ~key ~label ~active ~hidden ?(style = Style.empty) children =
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
  { (column ~key ~style children) with kind = Panel; text = label }
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
        ~on_toggle:(fun () -> on_request (Disclosure.Request.Toggle id))
        children)
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
        | Some (Tree_item _) ->
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
            | Tree _ ) -> column ~key ~style:row_style [ view ])
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
  ; kind
  ; text = initial_text
  ; style
  ; on_click = None
  ; editor = Some { controller; config; on_event }
  ; choice = None
  ; combobox = None
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
  ; control = None
  ; children = []
  }
;;

let radio_group ?key ?(style = Style.empty) ~config ~on_select () =
  { key
  ; kind = Radio_group
  ; text = ""
  ; style
  ; on_click = None
  ; editor = None
  ; control = None
  ; choice = Some { config; appearance = None; on_select }
  ; combobox = None
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
  ; children = []
  }
;;

let tab_bar ?key ?style ~config ~on_select () =
  { (radio_group ?key ?style ~config ~on_select ()) with kind = Tab_bar }
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
  ; choice = Some { config; appearance = Some appearance; on_select }
  }
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
  ; kind = Combobox
  ; text = initial_text
  ; style
  ; on_click = None
  ; editor = None
  ; control = None
  ; choice = None
  ; combobox = Some { controller; config; appearance; on_event }
  ; overlay = None
  ; tooltip = None
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; input_region = None
  ; highlight_scope = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; loading = None
  ; avatar = None
  ; rating = None
  ; slider = None
  ; number_input = None
  ; otp_input = None
  ; color_input = None
  ; calendar = None
  ; animation = None
  ; animation_program = None
  ; navigation_stack = None
  ; carousel = None
  ; container_query = None
  ; accessibility = None
  ; image = None
  ; extension = None
  ; split_pane = None
  ; document = None
  ; canvas = None
  ; chart = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; virtual_list = None
  ; table_cell = None
  ; children = []
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

let slider ?(style = Style.empty) ~controller ~config ~initial ~on_event () =
  { (text ~key:controller ~style "") with
    kind = Slider
  ; slider = Some { controller; config; initial; on_event }
  }
;;

let number_input ?(style = Style.empty) ~controller ~config ~initial ~on_event () =
  { (text ~key:controller ~style "") with
    kind = Number_input
  ; number_input = Some { controller; config; initial; on_event }
  }
;;

let otp_input ?(style = Style.empty) ~controller ~config ~initial ~on_event () =
  { (text ~key:controller ~style "") with
    kind = Otp_input
  ; otp_input = Some { controller; config; initial; on_event }
  }
;;

let color_input ?(style = Style.empty) ~controller ~config ~initial ~on_event () =
  { (text ~key:controller ~style "") with
    kind = Color_input
  ; color_input = Some { controller; config; initial; on_event }
  }
;;

let calendar
      ?(style = Style.empty)
      ~controller
      ~config
      ~initial
      ~initial_month
      ~on_event
      ()
  =
  { (text ~key:controller ~style "") with
    kind = Calendar
  ; calendar = Some { controller; config; initial; initial_month; on_event }
  }
;;

let rating ?key ?(style = Style.empty) ~config ~on_request () =
  { (text ?key ~style "") with kind = Rating; rating = Some { config; on_request } }
;;

let avatar ?key ?(style = Style.empty) ?on_change config =
  { (text ?key ~style "") with
    kind = Avatar
  ; avatar = Some config
  ; image =
      Option.map (Avatar.Expert.image config) ~f:(fun config -> { config; on_change })
  }
;;

let loading ?key ?(style = Style.empty) ~config () =
  { (text ?key ~style "") with kind = Loading; loading = Some config }
;;

let progress ?key ?(style = Style.empty) ~config () =
  { (text ?key ~style "") with kind = Progress; progress = Some config }
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

  let managed_table
        ?key
        ?source_key
        ?style
        ?(commands = [])
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
          column
            ~key
            (List.map cells ~f:(fun (metadata, child) ->
               { (column
                    ~key:
                      (Key.of_string_exn
                         (Table_column.Id.to_string (Table.Cell.column metadata)))
                    [ child ])
                 with
                 table_cell = Some metadata
               })))
      in
      { root with
        children
      ; virtual_list =
          Option.map root.virtual_list ~f:(fun list ->
            { list with
              table = Some { source_key; config; query_generation; commands; on_input }
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
    }

  type nonrec 'action slider = 'action slider =
    { controller : Key.t
    ; config : Slider.Config.t
    ; initial : Slider.Value.t
    ; on_event : Slider.Event.t -> 'action
    }

  type nonrec 'action number_input = 'action number_input =
    { controller : Key.t
    ; config : Number_input.Config.t
    ; initial : Number_input.Value.t
    ; on_event : Number_input.Event.t -> 'action
    }

  type nonrec 'action otp_input = 'action otp_input =
    { controller : Key.t
    ; config : Otp_input.Config.t
    ; initial : Otp_input.Value.t
    ; on_event : Otp_input.Event.t -> 'action
    }

  type nonrec 'action color_input = 'action color_input =
    { controller : Key.t
    ; config : Color_input.Config.t
    ; initial : Color_value.Value.t
    ; on_event : Color_input.Event.t -> 'action
    }

  type nonrec 'action calendar = 'action calendar =
    { controller : Key.t
    ; config : Calendar.Config.t
    ; initial : Calendar.Selection.t
    ; initial_month : Calendar.Month.t
    ; on_event : Calendar.Event.t -> 'action
    }

  type nonrec 'action rating = 'action rating =
    { config : Rating.Config.t
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
    ; on_select : Choice.Id.t -> 'action
    }

  type nonrec 'action editor = 'action editor =
    { controller : Key.t
    ; config : Text_input.Config.t
    ; on_event : Text_input.Event.t -> 'action
    }

  type nonrec 'action palette = 'action palette =
    { config : Command_palette.Config.t
    ; appearance : Command_palette.Appearance.t
    ; on_dismiss : Command_palette.Dismissal.t -> 'action
    }

  type nonrec menu = menu =
    { presentation : Menu.Expert.presentation
    ; menus : Menu.t list
    ; appearance : Menu.Appearance.t
    }

  type 'action description = 'action t =
    { key : Key.t option
    ; kind : Kind.t
    ; text : string
    ; style : Style.t
    ; on_click : (unit -> 'action) option
    ; editor : 'action editor option
    ; control : Control.t option
    ; choice : 'action choice option
    ; combobox : 'action combobox option
    ; overlay : 'action overlay option
    ; tooltip : 'action tooltip option
    ; commands : 'action Ui_command.Registry.t option
    ; command_ref : Ui_command.Id.t option
    ; drag_source : 'action drag_source option
    ; drop_target : 'action drop_target option
    ; pointer : 'action pointer option
    ; input_region : 'action input_region option
    ; highlight_scope : 'action highlight_scope option
    ; notification : 'action notification option
    ; toast_stack : Toast.Stack.t option
    ; progress : Progress.Config.t option
    ; loading : Loading.Config.t option
    ; avatar : Avatar.Config.t option
    ; rating : 'action rating option
    ; slider : 'action slider option
    ; number_input : 'action number_input option
    ; otp_input : 'action otp_input option
    ; color_input : 'action color_input option
    ; calendar : 'action calendar option
    ; animation : 'action animation option
    ; animation_program : 'action animation_program option
    ; navigation_stack : Gpuio_protocol.Navigation_stack_wire.Config.t option
    ; carousel :
        (Gpuio_protocol.Carousel_wire.Config.t * (Carousel.Request.t -> 'action)) option
    ; container_query : 'action container_query option
    ; accessibility : Accessibility.t option
    ; image : 'action image option
    ; extension : 'action extension option
    ; split_pane : 'action split_pane option
    ; document : 'action document option
    ; canvas : 'action canvas option
    ; chart : 'action chart option
    ; palette : 'action palette option
    ; menu : menu option
    ; focus_scope : Focus_scope.t option
    ; virtual_list : 'action virtual_list option
    ; table_cell : Table.Cell.t option
    ; children : 'action t list
    }

  let describe t = t
end
