open Core
module Wire = Gpuio_protocol.Checkable_wire.Tab_order

type t = Wire.t [@@deriving equal, sexp_of]

let of_wire t =
  if Wire.valid t
  then Ok t
  else Or_error.error_string "Tab index must be in -1000000..1000000"
;;

let create ?(tab_stop = true) ?(index = 0) () =
  of_wire { Wire.tab_stop; index = Int64.of_int index }
;;

let default = { Wire.tab_stop = true; index = 0L }
let tab_stop (t : t) = t.tab_stop
let index (t : t) = Int64.to_int_exn t.index

module Expert = struct
  let to_wire t = t
  let of_wire = of_wire
end
