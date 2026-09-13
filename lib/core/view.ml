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

type 'action pointer =
  { config : Pointer.Config.t
  ; on_event : Pointer.Event.t -> 'action
  }

type 'action notification =
  { config : Toast.Config.t
  ; on_dismiss : Toast.Dismissal.t -> 'action
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
  ; notification : 'action notification option
  ; toast_stack : Toast.Stack.t option
  ; progress : Progress.Config.t option
  ; palette : 'action palette option
  ; menu : menu option
  ; focus_scope : Focus_scope.t option
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; control = None
  ; children = []
  }
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

let button ?key ?(style = Style.empty) ?accessible_name ?(disabled = false) ~on_click text
  =
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; control = Some (Button { disabled })
  ; children = []
  }
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; control = None
  ; children
  }
;;

let focus_scope ?key ?style ~config children =
  { (container ?key ?style [] children) with
    kind = Focus_scope
  ; commands = None
  ; command_ref = None
  ; drag_source = None
  ; drop_target = None
  ; pointer = None
  ; notification = None
  ; toast_stack = None
  ; progress = None
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

let dialog ?key ?style ~config ~on_dismiss content =
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
      overlay = Some { kind = Dialog; config; on_dismiss }
    }
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

let command_button ?key ?style ~command () =
  let style = button_style (Option.value style ~default:Style.empty) in
  { (text ?key ~style "") with
    kind = Command_button
  ; on_click = None
  ; control = None
  ; command_ref = Some command
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

let row ?key ?style children =
  container ?key ?style [ Display Flex; Direction Row ] children
;;

let column ?key ?style children =
  container ?key ?style [ Display Flex; Direction Column ] children
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
  ; children = []
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
  ; notification = None
  ; toast_stack = None
  ; progress = None
  ; palette = None
  ; menu = None
  ; focus_scope = None
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
  type nonrec 'action drag_source = 'action drag_source =
    { config : Drag_and_drop.Source.t
    ; on_event : Drag_and_drop.Source_event.t -> 'action
    }

  type nonrec 'action drop_target = 'action drop_target =
    { config : Drag_and_drop.Target.t
    ; on_event : Drag_and_drop.Target_event.t -> 'action
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
    ; notification : 'action notification option
    ; toast_stack : Toast.Stack.t option
    ; progress : Progress.Config.t option
    ; palette : 'action palette option
    ; menu : menu option
    ; focus_scope : Focus_scope.t option
    ; children : 'action t list
    }

  let describe t = t
end
