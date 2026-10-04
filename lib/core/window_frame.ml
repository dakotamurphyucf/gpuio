open Core
module Wire = Gpuio_protocol.Window_wire.Frame

type t = Wire.t [@@deriving equal, sexp_of]

let create ?(shadow_size = 20.) ?(resize_hit_size = 4.) () =
  let config = { Wire.shadow_size; resize_hit_size } in
  if Wire.valid config
  then Ok config
  else
    Or_error.error_string
      "window frame needs shadow margin 0..128 and resize half-band 0.5..32"
;;

let default = Wire.default

module Expert = struct
  let to_wire t = t
end
