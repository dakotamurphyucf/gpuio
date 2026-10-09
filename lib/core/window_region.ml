open Core
module Wire = Gpuio_protocol.Window_region_wire
module Edge = Wire.Edge

type t = Wire.t =
  | Title_bar
  | Exclude
  | Resize of Edge.t
[@@deriving equal, sexp_of]

module Expert = struct
  let to_wire t = t
end
