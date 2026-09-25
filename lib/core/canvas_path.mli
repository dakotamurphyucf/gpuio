open Core

module Command : sig
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
end

(** Immutable local-space path. A contour starts with Move, then one or more
    drawing commands, optionally Close. A closed contour requires a new Move
    before another drawing command. Empty contours and >4096 commands fail.
    Control points share the geometry coordinate bounds.

    Open contours are suitable for strokes. Filled resources require every
    contour explicitly closed; the canvas will not silently close open paths.
    Admission does not promise successful tessellation: self-intersections and
    highly detailed curves remain subject to native tessellation budgets. *)
type t [@@deriving equal, sexp_of]

val create : Command.t list -> t Or_error.t
val command_count : t -> int
val is_closed : t -> bool

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Canvas_wire.Path.t
end
