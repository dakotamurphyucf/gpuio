open Core

type t =
  { wire : Gpuio_protocol.Editor_frame_wire.t
  ; loading_label : string
  ; show_password_label : string
  ; hide_password_label : string
  }
[@@deriving equal, sexp_of]

let create
      ?clear_label
      ?(loading = false)
      ?(loading_label = "Loading")
      ?(show_password_label = "Show password")
      ?(hide_password_label = "Hide password")
      ?(gap = 6.)
      ()
  =
  let labels =
    Option.to_list clear_label
    @ [ loading_label; show_password_label; hide_password_label ]
  in
  if
    List.exists labels ~f:(fun label ->
      String.is_empty (String.strip label)
      || String.length label > 4096
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000')
  then
    Or_error.error_string
      "input frame labels must be nonblank UTF-8 without NUL, at most 4096 bytes"
  else if (not (Float.is_finite gap)) || Float.(gap < 0. || gap > 256.)
  then Or_error.error_string "input frame gap must be finite and in 0..256 logical pixels"
  else
    Ok
      { wire = { clear_label; loading; gap }
      ; loading_label
      ; show_password_label
      ; hide_password_label
      }
;;

let default = create () |> Or_error.ok_exn

module Expert = struct
  let to_wire t = t.wire
  let is_loading t = t.wire.loading
  let loading_label t = t.loading_label

  let reveal_label t ~revealed =
    if revealed then t.hide_password_label else t.show_password_label
  ;;
end
