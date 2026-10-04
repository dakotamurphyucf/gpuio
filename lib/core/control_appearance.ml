open Core
module W = Gpuio_protocol.Wire.Control_appearance
module Label_position = W.Label_position

type t =
  { size : float
  ; switch_width : float
  ; gap : float
  ; label_position : Label_position.t
  ; indicator_style : Style.t
  ; mark_style : Style.t
  }
[@@deriving equal, sexp_of]

let create
      ?(size = 18.)
      ?switch_width
      ?(gap = 8.)
      ?(label_position = Label_position.After)
      ?(indicator_style = Style.empty)
      ?(mark_style = Style.empty)
      ()
  =
  let switch_width = Option.value switch_width ~default:(size *. (5. /. 3.)) in
  let bounded value ~min:minimum ~max:maximum =
    Float.is_finite value && Float.(value >= minimum && value <= maximum)
  in
  let open Or_error.Let_syntax in
  let%bind () =
    if
      bounded size ~min:8. ~max:128.
      && bounded switch_width ~min:size ~max:256.
      && bounded gap ~min:0. ~max:128.
    then Ok ()
    else Or_error.error_string "invalid checkable control geometry"
  in
  let states = [ Style.State.Base; Checked; Indeterminate; Disabled ] in
  let%bind () =
    Style.Expert.validate_scope
      indicator_style
      ~states
      ~properties:
        [ Background
        ; Foreground
        ; Opacity
        ; Border_color
        ; Border_top_width
        ; Border_right_width
        ; Border_bottom_width
        ; Border_left_width
        ; Top_left_radius
        ; Top_right_radius
        ; Bottom_left_radius
        ; Bottom_right_radius
        ]
  in
  let%bind () =
    Style.Expert.validate_scope mark_style ~states ~properties:[ Foreground; Opacity ]
  in
  let%map () =
    if
      Style.Expert.declaration_count indicator_style
      + Style.Expert.declaration_count mark_style
      <= 128
    then Ok ()
    else Or_error.error_string "control appearance exceeds 128 declarations"
  in
  { size; switch_width; gap; label_position; indicator_style; mark_style }
;;

let default = create () |> Or_error.ok_exn
let size t = t.size
let switch_width t = t.switch_width
let gap t = t.gap
let label_position t = t.label_position

module Expert = struct
  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let%bind indicator_style = Style.Expert.to_wire t.indicator_style ~theme in
    let%map mark_style = Style.Expert.to_wire t.mark_style ~theme in
    ({ size = t.size
     ; switch_width = t.switch_width
     ; gap = t.gap
     ; label_position = t.label_position
     ; indicator_style
     ; mark_style
     }
     : W.t)
  ;;
end
