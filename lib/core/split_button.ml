open Core

module Appearance = struct
  type t =
    { surface : Style.t
    ; menu_open : Style.t
    }
  [@@deriving equal, sexp_of]

  let create ?(surface = Style.empty) ?(menu_open = Style.empty) () =
    let open Or_error.Let_syntax in
    let validate style =
      Style.Expert.validate_scope
        style
        ~states:[ Base ]
        ~properties:[ Background; Foreground; Border_color; Shadows; Text_decoration ]
    in
    let%bind () = validate surface in
    let%bind () = validate menu_open in
    let%map () =
      if
        Style.Expert.declaration_count surface + Style.Expert.declaration_count menu_open
        <= 64
      then Ok ()
      else Or_error.error_string "split button appearance exceeds 64 declarations"
    in
    { surface; menu_open }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Expert = struct
  let to_wire (t : Appearance.t) ~parts ~theme =
    let open Or_error.Let_syntax in
    let%bind surface = Style.Expert.to_wire t.surface ~theme in
    let%map menu_open = Style.Expert.to_wire t.menu_open ~theme in
    ({ parts; surface; menu_open } : Gpuio_protocol.Wire.Split_button.t)
  ;;
end
