open Core

module Kind = struct
  type t =
    | Container
    | Text
    | Button
    | Input
    | Textarea
  [@@deriving equal, sexp_of]
end

type 'action editor =
  { controller : Key.t
  ; config : Text_input.Config.t
  ; on_event : Text_input.Event.t -> 'action
  }

type 'action t =
  { key : Key.t option
  ; kind : Kind.t
  ; text : string
  ; style : Style.t
  ; on_click : (unit -> 'action) option
  ; editor : 'action editor option
  ; children : 'action t list
  }

let text ?key ?(style = Style.empty) text =
  { key; kind = Text; text; style; on_click = None; editor = None; children = [] }
;;

let button ?key ?(style = Style.empty) ?accessible_name ~on_click text =
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
  ; on_click = Some on_click
  ; editor = None
  ; children = []
  }
;;

let container ?key ?(style = Style.empty) defaults children =
  { key
  ; kind = Container
  ; text = ""
  ; style = Style.merge [ Style.create_exn defaults; style ]
  ; on_click = None
  ; editor = None
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
  ; children = []
  }
;;

module Expert = struct
  module Kind = Kind

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
    ; children : 'action t list
    }

  let describe t = t
end
