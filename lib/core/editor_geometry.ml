open Core
module W = Gpuio_protocol.Editor_geometry_wire

type t = W.t [@@deriving equal, sexp_of]

let revision t = Text_input.Revision.of_int64 t.W.revision |> Or_error.ok_exn
let x t = t.W.x
let y t = t.W.y
let width t = t.W.width
let height t = t.W.height

module Expert = struct
  let of_wire t =
    if W.is_valid t then Ok t else Or_error.error_string "invalid editor range geometry"
  ;;
end
