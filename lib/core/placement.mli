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

module Corner : sig
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving equal, sexp_of]
end

(** Placement of a popup surface. Coordinates, offsets and margins use logical
    window-content pixels, not screen/device pixels. Native placement uses the
    current viewport and measured popup size. *)
type t [@@deriving equal, sexp_of]

val default : t

(** Defaults to Bottom/Start, zero offset and eight-pixel viewport margin.
    [offset] is a signed gap in -16384..16384; negative values allow overlap.
    Native layout flips the preferred side when needed, then clamps the origin.
    [viewport_margin] is finite and in 0..16384, added to the native client inset.
    An oversized margin is capped at half of each viewport dimension. Oversized
    content keeps the leading margin; this operation does not resize content. *)
val create
  :  ?side:Side.t
  -> ?align:Align.t
  -> ?offset:float
  -> ?viewport_margin:float
  -> unit
  -> t Or_error.t

(** Place the selected popup corner at a window point, then clamp without
    flipping. Defaults: Top_left and eight-pixel viewport margin. [x] and [y]
    must be finite and in -1_000_000..1_000_000. The trigger remains the activation
    and focus-return anchor; its geometry does not determine this position. *)
val at_point
  :  ?corner:Corner.t
  -> ?viewport_margin:float
  -> x:float
  -> y:float
  -> unit
  -> t Or_error.t

module Expert : sig
  (** The legacy side record must be paired with [geometry] when constructing
      protocol operations. Use ordinary View helpers to submit both together. *)
  val to_wire : t -> Gpuio_protocol.Wire.Placement.t

  val geometry : t -> Gpuio_protocol.Placement_geometry_wire.t option
end
