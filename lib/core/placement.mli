open Core

module Side : sig
  type t =
    | Top
    | Right
    | Bottom
    | Left
  [@@deriving equal, sexp_of]
end

module Align : sig
  type t =
    | Start
    | Center
    | End
  [@@deriving equal, sexp_of]
end

(** Placement of an anchored surface. Native layout flips the preferred side
    when necessary and clamps the result inside the current viewport. *)
type t [@@deriving equal, sexp_of]

val default : t

(** Defaults to Bottom/Start with zero offset. [offset] is a signed logical-pixel
    gap from the anchor; negative values allow overlap. It must be finite and
    within -16384..16384. Alignment follows the anchor's cross axis. *)
val create : ?side:Side.t -> ?align:Align.t -> ?offset:float -> unit -> t Or_error.t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Wire.Placement.t
end
