open Core
module W = Gpuio_protocol.Text_area_layout_wire

module Wrapping_indent = struct
  type t = W.Wrapping_indent.t =
    | Flush_left
    | Match_first_line
  [@@deriving equal, sexp_of]
end

type t = W.t [@@deriving equal, sexp_of]

let create
      ?(soft_wrap = true)
      ?(wrapping_indent = Wrapping_indent.Match_first_line)
      ?(show_whitespace = false)
      ?cursor_margin_lines
      ()
  =
  let t : t =
    { soft_wrap
    ; wrapping_indent
    ; show_whitespace
    ; cursor_margin_lines = Option.map cursor_margin_lines ~f:Int64.of_int
    }
  in
  if W.is_valid t
  then Ok t
  else Or_error.error_string "cursor margin must be in 0..256 line heights"
;;

let default = create () |> Or_error.ok_exn
let soft_wrap t = t.W.soft_wrap
let wrapping_indent t = t.W.wrapping_indent
let show_whitespace t = t.W.show_whitespace
let cursor_margin_lines t = Option.map t.W.cursor_margin_lines ~f:Int64.to_int_exn

module Expert = struct
  let to_wire (t : t) = t
end
