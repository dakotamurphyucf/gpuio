open Core
module W = Gpuio_protocol.Editor_viewport_wire

module Offset = struct
  type t = W.Offset.t [@@deriving equal, sexp_of]

  let create ~x ~y =
    let t : t = { x; y } in
    if W.Offset.is_valid t
    then Ok t
    else
      Or_error.error_string
        "editor scroll offsets must be finite logical pixels in 0..1e9"
  ;;

  let origin = create ~x:0. ~y:0. |> Or_error.ok_exn
  let x t = t.W.Offset.x
  let y t = t.W.Offset.y
end

type t = W.t [@@deriving equal, sexp_of]

let offset t = t.W.offset
let width t = t.W.width
let height t = t.W.height
let line_height t = t.W.line_height
let first_buffer_line t = Int64.to_int_exn t.W.first_buffer_line
let buffer_line_limit t = Int64.to_int_exn t.W.buffer_line_limit

module Expert = struct
  let offset_to_wire (t : Offset.t) = t

  let of_wire t =
    if W.is_valid t
    then Ok t
    else Or_error.error_string "invalid editor viewport observation"
  ;;
end
