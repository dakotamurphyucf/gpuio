open Core
module W = Gpuio_protocol.Numeric_wire
module Direction = W.Direction

module Domain = struct
  type t = W.Domain.t [@@deriving equal, sexp_of]

  let validate t =
    if W.Domain.valid t
    then Ok t
    else
      Or_error.error_string
        "numeric domain requires finite ordered bounds and a resolvable positive step"
  ;;

  let create ~min ~max ~step = validate { W.Domain.min; max; step }
  let min t = t.W.Domain.min
  let max t = t.W.Domain.max
  let step t = t.W.Domain.step
  let contains = W.Domain.contains

  let result = function
    | Some value -> Ok value
    | None -> Or_error.error_string "numeric operation requires a finite value"
  ;;

  let normalize t value = W.Domain.normalize t value |> result
  let advance t value ~direction = W.Domain.advance t value ~direction |> result
end

module Draft = W.Draft

module Expert = struct
  let to_wire t = t
  let of_wire = Domain.validate
end
