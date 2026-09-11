open Core
module Wire = Gpuio_protocol.Wire
type t = Wire.Length.t [@@deriving equal, sexp_of]
let valid value = Float.is_finite value && Float.(abs value <= 1_000_000.)
let px value = if valid value then Ok (Wire.Length.Px value) else Or_error.error_string "invalid logical pixel length"
let px_exn value = px value |> Or_error.ok_exn
let percent value = if valid value then Ok (Wire.Length.Percent value) else Or_error.error_string "invalid percentage length"
let percent_exn value = percent value |> Or_error.ok_exn
let auto = Wire.Length.Auto
module Expert = struct let to_wire t = t end
