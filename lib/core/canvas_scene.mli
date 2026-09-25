open Core
module Geometry = Canvas_geometry
module Resource = Canvas_resource

module Item_id : sig
  type t [@@deriving equal, compare, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Stroke : sig
  type t

  (** Width is finite and in (0,256] logical pixels, in local coordinates. *)
  val create : color:Color.t -> width:float -> t Or_error.t
end

module Paint : sig
  type t

  (** At least one of fill/stroke is required. Colors resolve when creating the
      scene; an unknown theme token fails construction. *)
  val create : ?fill:Color.t -> ?stroke:Stroke.t -> unit -> t Or_error.t
end

module Drawing : sig
  type t

  val rectangle : Geometry.Rect.t -> paint:Paint.t -> t
  val ellipse : Geometry.Rect.t -> paint:Paint.t -> t

  (** Filled paths must have every contour explicitly closed. *)
  val path : Resource.path Resource.t -> paint:Paint.t -> t Or_error.t

  val text : Resource.text Resource.t -> origin:Geometry.Point.t -> color:Color.t -> t

  (** The full source is stretched into the destination rectangle. Image decode
      failure is native; construction neither decodes nor accesses files. *)
  val image : Resource.image Resource.t -> bounds:Geometry.Rect.t -> t
end

module Interaction : sig
  type t

  (** Nonblank UTF-8 label, <=1024 bytes, without NUL. Hit regions are local;
      draggable/activatable default to false. Every interactive item is selectable.
      These are native policies; no synchronous OCaml callback is stored here. *)
  val create
    :  label:string
    -> hit_region:Geometry.Hit_region.t
    -> ?draggable:bool
    -> ?activatable:bool
    -> unit
    -> t Or_error.t
end

module Item : sig
  type t

  (** Local-to-world transform defaults to identity. Up to eight world-space clip
      rectangles intersect. Geometry and hit regions must remain in the coordinate
      domain after transformation. Text supports translation/positive uniform
      scale; images translation/positive axis scales. Shapes/paths support affine
      transforms. Unsupported combinations fail instead of approximating them. *)
  val create
    :  id:Item_id.t
    -> ?transform:Geometry.Transform.t
    -> ?clips:Geometry.Rect.t list
    -> ?interaction:Interaction.t
    -> Drawing.t
    -> t Or_error.t

  val id : t -> Item_id.t
end

(** Immutable scene snapshot. Item order is back-to-front. Resources are collected
    automatically from drawings and shared by ID; conflicting generations/data
    under the same ID fail. Item IDs must be unique. Limits: 20,000 items, 4,096
    resources, 65,536 path commands, 1 MiB total text, 2,048 interactive items,
    4 MiB encoded scene. No native allocation occurs during construction.

    Image handles retain their application identity until the adapter checks it.
    They do not extend asset registration lifetime. Registration/set/reset/release
    belong to the Eio adapter, rather than these pure values. *)
type t

val create : description:string -> ?theme:Theme.t -> Item.t list -> t Or_error.t
val item_count : t -> int
val resource_count : t -> int
val encoded_bytes : t -> int

(** Snapshot identity for reactive cutoffs. Independently constructed scenes are
    distinct even if their contents happen to agree. *)
val equal : t -> t -> bool

module Handle : sig
  (** Borrowed identity of a registered native scene in one application. Holding
      a handle does not extend its scope or allow acquisition after release. *)
  type t [@@deriving equal, sexp_of]
end

module Expert : sig
  module Owner : sig
    type t

    val create : unit -> t
    val equal : t -> t -> bool
  end

  val handle : owner:Owner.t -> Gpuio_protocol.Resource_id.t -> Handle.t
  val belongs_to : Handle.t -> owner:Owner.t -> bool
  val native_id : Handle.t -> Gpuio_protocol.Resource_id.t

  (** Validate every image's application owner before emitting native handles. *)
  val encode : t -> asset_owner:Asset.Expert.Owner.t -> string Or_error.t

  val assets_belong_to : t -> asset_owner:Asset.Expert.Owner.t -> bool
end
