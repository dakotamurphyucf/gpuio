open Core

module Position = struct
  module Wire = Gpuio_protocol.Checkable_wire.Position

  type t = Wire.t [@@deriving equal, sexp_of]

  let of_wire t =
    if Wire.valid t
    then Ok t
    else
      Or_error.error_string
        "Radio position requires 0 <= index < count and count in 1..100000"
  ;;

  let create ~index ~count =
    of_wire { Wire.index = Int64.of_int index; count = Int64.of_int count }
  ;;

  let index (t : t) = Int64.to_int_exn t.index
  let count (t : t) = Int64.to_int_exn t.count

  module Expert = struct
    let to_wire t = t
    let of_wire = of_wire
  end
end
