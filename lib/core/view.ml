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
  ; on_select : Choice.Id.t -> 'action
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
  ; children : 'action t list
  }

let text ?key ?(style = Style.empty) text =
  { key
  ; kind = Text
  ; text
  ; style
  ; on_click = None
  ; editor = None
  ; choice = None
  ; control = None
  ; children = []
  }
;;

let button ?key ?(style = Style.empty) ?accessible_name ?(disabled = false) ~on_click text
  =
  let style =
    match accessible_name with
    | None -> style
    | Some name -> Style.merge [ style; Style.create_exn [ Accessible_name name ] ]
  in
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
  { key
  ; kind = Button
  ; text
  ; style = Style.merge [ defaults; style ]
  ; on_click = (if disabled then None else Some on_click)
  ; editor = None
  ; choice = None
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
  ; control = None
  ; children
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
  ; choice = Some { config; on_select }
  ; children = []
  }
;;

let select ?key ?(style = Style.empty) ~config ~on_select () =
  { (radio_group ?key ~style ~config ~on_select ()) with kind = Select }
;;

module Expert = struct
  module Kind = Kind
  module Control = Control

  type nonrec 'action choice = 'action choice =
    { config : Choice.Config.t
    ; on_select : Choice.Id.t -> 'action
    }

  type nonrec 'action editor = 'action editor =
    { controller : Key.t
    ; config : Text_input.Config.t
    ; on_event : Text_input.Event.t -> 'action
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
    ; children : 'action t list
    }

  let describe t = t
end
