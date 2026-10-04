open Core

let validate style ~states =
  let open Or_error.Let_syntax in
  let%bind () =
    Style.Expert.validate_scope
      style
      ~states
      ~properties:
        [ Background
        ; Foreground
        ; Border_color
        ; Border_style
        ; Top_left_radius
        ; Top_right_radius
        ; Bottom_left_radius
        ; Bottom_right_radius
        ; Shadows
        ; Font_size
        ; Font_family
        ; Font_weight
        ; Text_decoration
        ]
  in
  if Style.Expert.declaration_count style <= 128
  then Ok style
  else Or_error.error_string "table presentation exceeds 128 declarations"
;;

module Header = struct
  type t = Style.t [@@deriving equal, sexp_of]

  let create style = validate style ~states:[ Base; Hovered; Pressed; Disabled ]
  let empty = Style.empty
  let style t = t
end

module Row = struct
  type t = Style.t [@@deriving equal, sexp_of]

  let create style =
    validate style ~states:[ Base; Focused; Hovered; Pressed; Disabled; Selected ]
  ;;

  let empty = Style.empty
  let style t = t
end
