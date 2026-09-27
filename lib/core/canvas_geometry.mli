open Core

(** Retained-canvas geometry in logical pixels. All admitted coordinates are
    finite and within +/-1,000,000. Device scale is applied only by GPUI.
    Constructors and calculated results validate their bounds. Hit containment
    uses a 1e-7 logical-pixel boundary tolerance in the tested coordinate space;
    this absorbs floating-point roundoff, not a device-pixel hit slop. *)
module Point : sig
  type t [@@deriving equal, sexp_of]

  val create : x:float -> y:float -> t Or_error.t
  val x : t -> float
  val y : t -> float
end

module Rect : sig
  type t [@@deriving equal, sexp_of]

  (** Width and height must be positive, <=1,000,000. Both corners must fit the
      coordinate domain. Contains includes the boundary. *)
  val create : x:float -> y:float -> width:float -> height:float -> t Or_error.t

  val x : t -> float
  val y : t -> float
  val width : t -> float
  val height : t -> float
  val contains : t -> Point.t -> bool

  (** Empty intersections, including edge-only contact, return [None]. *)
  val intersect : t -> t -> t option
end

module Transform : sig
  type t [@@deriving equal, sexp_of]

  val identity : t

  (** Maps (x,y) to (a*x+c*y+tx, b*x+d*y+ty). Linear coefficients must be
      finite and within +/-1000, translation within the coordinate domain,
      and absolute determinant >=1e-8. Reflections are supported. *)
  val create
    :  a:float
    -> b:float
    -> c:float
    -> d:float
    -> tx:float
    -> ty:float
    -> t Or_error.t

  val translate : x:float -> y:float -> t Or_error.t
  val scale : x:float -> y:float -> t Or_error.t
  val rotate : radians:float -> t Or_error.t

  (** [compose parent ~local] applies local first, then parent. Composition can
      exceed the admission bounds even when both inputs are valid. *)
  val compose : t -> local:t -> t Or_error.t

  val apply : t -> Point.t -> Point.t Or_error.t

  (** Map a world point back to local coordinates. The inverse matrix is a
      calculation detail; the resulting point must still fit the domain. *)
  val unapply : t -> Point.t -> Point.t Or_error.t
end

module Hit_region : sig
  type t [@@deriving equal, sexp_of]

  val rectangle : Rect.t -> t
  val ellipse : Rect.t -> t

  (** Even-odd polygon, 3..256 points, with at least one non-collinear triple
      (absolute cross product >1e-8). An explicit repeated closing point is
      optional. Self-intersections use the even-odd rule. Edges count as inside
      with a 1e-7 logical-pixel tolerance per local coordinate axis. *)
  val polygon : Point.t list -> t Or_error.t

  val contains : t -> Point.t -> bool

  (** Clip rectangles are in world coordinates. Inverse transformation precedes
      local containment; clips intersect. Empty clips mean no world clipping. *)
  val hit : t -> transform:Transform.t -> clips:Rect.t list -> Point.t -> bool
end

module Expert : sig
  val point_to_wire : Point.t -> Gpuio_protocol.Canvas_wire.Point.t
  val rect_to_wire : Rect.t -> Gpuio_protocol.Canvas_wire.Rect.t
  val transform_to_wire : Transform.t -> Gpuio_protocol.Canvas_wire.Transform.t
  val hit_region_to_wire : Hit_region.t -> Gpuio_protocol.Canvas_wire.Hit_region.t
end
