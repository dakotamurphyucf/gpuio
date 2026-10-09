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

module Corner = struct
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving equal, sexp_of]
end

module Strategy = struct
  type t =
    | Side of
        { side : Side.t
        ; align : Align.t
        ; offset : float
        }
    | Point of
        { corner : Corner.t
        ; x : float
        ; y : float
        }
  [@@deriving equal, sexp_of]
end

type t =
  { strategy : Strategy.t
  ; viewport_margin : float
  }
[@@deriving equal, sexp_of]

let default =
  { strategy = Side { side = Bottom; align = Start; offset = 0. }; viewport_margin = 8. }
;;

let valid_margin n = Float.is_finite n && Float.(n >= 0. && n <= 16384.)

let create
      ?(side = Side.Bottom)
      ?(align = Align.Start)
      ?(offset = 0.)
      ?(viewport_margin = 8.)
      ()
  =
  if not (valid_margin viewport_margin)
  then
    Or_error.error_string
      "placement viewport margin must be finite and in 0..16384 logical pixels"
  else if Float.is_finite offset && Float.(offset >= -16384. && offset <= 16384.)
  then Ok { strategy = Side { side; align; offset }; viewport_margin }
  else
    Or_error.error_string
      "placement offset must be finite and in -16384..16384 logical pixels"
;;

let at_point ?(corner = Corner.Top_left) ?(viewport_margin = 8.) ~x ~y () =
  let valid_coordinate n =
    Float.is_finite n && Float.(n >= -1_000_000. && n <= 1_000_000.)
  in
  if not (valid_margin viewport_margin)
  then
    Or_error.error_string
      "placement viewport margin must be finite and in 0..16384 logical pixels"
  else if valid_coordinate x && valid_coordinate y
  then Ok { strategy = Point { corner; x; y }; viewport_margin }
  else
    Or_error.error_string
      "placement coordinates must be finite and in -1000000..1000000 logical pixels"
;;

module Expert = struct
  let to_wire t : Gpuio_protocol.Wire.Placement.t =
    let side, align, offset =
      match t.strategy with
      | Side { side; align; offset } -> side, align, offset
      | Point _ -> Side.Bottom, Align.Start, 0.
    in
    { side =
        (match side with
         | Top -> Top
         | Right -> Right
         | Bottom -> Bottom
         | Left -> Left)
    ; align =
        (match align with
         | Start -> Start
         | Center -> Center
         | End -> End)
    ; offset
    }
  ;;

  let geometry t : Gpuio_protocol.Placement_geometry_wire.t option =
    let point =
      match t.strategy with
      | Side _ -> None
      | Point { corner; x; y } ->
        Some
          { Gpuio_protocol.Placement_geometry_wire.Point.corner =
              (match corner with
               | Top_left -> Top_left
               | Top_right -> Top_right
               | Bottom_left -> Bottom_left
               | Bottom_right -> Bottom_right)
          ; x
          ; y
          }
    in
    if Option.is_none point && Float.equal t.viewport_margin 8.
    then None
    else Some { viewport_margin = t.viewport_margin; point }
  ;;
end
