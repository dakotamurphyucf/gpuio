open Core

module Side = struct
  type t =
    | Top
    | Right
    | Bottom
    | Left
  [@@deriving equal, sexp_of]
end

module Align = struct
  type t =
    | Start
    | Center
    | End
  [@@deriving equal, sexp_of]
end

type t =
  { side : Side.t
  ; align : Align.t
  ; offset : float
  }
[@@deriving equal, sexp_of]

let default = { side = Bottom; align = Start; offset = 0. }

let create ?(side = Side.Bottom) ?(align = Align.Start) ?(offset = 0.) () =
  if Float.is_finite offset && Float.(offset >= -16384. && offset <= 16384.)
  then Ok { side; align; offset }
  else
    Or_error.error_string
      "placement offset must be finite and in -16384..16384 logical pixels"
;;

module Expert = struct
  let to_wire t : Gpuio_protocol.Wire.Placement.t =
    { side =
        (match t.side with
         | Top -> Top
         | Right -> Right
         | Bottom -> Bottom
         | Left -> Left)
    ; align =
        (match t.align with
         | Start -> Start
         | Center -> Center
         | End -> End)
    ; offset = t.offset
    }
  ;;
end
