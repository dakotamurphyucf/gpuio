open Core
module Wire = Gpuio_protocol.Canvas_wire.Path

module Command = struct
  type t =
    | Move of Canvas_geometry.Point.t
    | Line of Canvas_geometry.Point.t
    | Quadratic of
        { control : Canvas_geometry.Point.t
        ; endpoint : Canvas_geometry.Point.t
        }
    | Cubic of
        { first_control : Canvas_geometry.Point.t
        ; second_control : Canvas_geometry.Point.t
        ; endpoint : Canvas_geometry.Point.t
        }
    | Close
  [@@deriving equal, sexp_of]

  let to_wire =
    let point = Canvas_geometry.Expert.point_to_wire in
    function
    | Move p -> Wire.Command.Move (point p)
    | Line p -> Line (point p)
    | Quadratic { control; endpoint } -> Quadratic (point control, point endpoint)
    | Cubic { first_control; second_control; endpoint } ->
      Cubic (point first_control, point second_control, point endpoint)
    | Close -> Close
  ;;
end

type t = Wire.t [@@deriving equal, sexp_of]

let create commands =
  if List.length commands > Wire.max_commands
  then Or_error.error_string "canvas path exceeds 4096 commands"
  else (
    let t = List.map commands ~f:Command.to_wire in
    if Wire.valid t
    then Ok t
    else
      Or_error.error_string "canvas path requires nonempty contours beginning with Move")
;;

let command_count = List.length
let is_closed = Wire.is_closed

module Expert = struct
  let to_wire t = t
end
