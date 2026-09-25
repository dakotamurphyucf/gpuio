open Core

module Id : sig
  type t [@@deriving equal, compare, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

type path
type text
type image

(** Immutable reusable drawing data. The phantom kind prevents using an image
    as a path or text resource. IDs are positive and scene-local; generations
    are positive and default to 1. Within one registered scene generation,
    changing a resource's contents requires a higher resource generation,
    including after removal and reintroduction. *)
type 'kind t

val id : _ t -> Id.t
val generation : _ t -> int64
val equal : 'kind t -> 'kind t -> bool
val sexp_of_t : _ t -> Sexp.t
val path : id:Id.t -> ?generation:int64 -> Canvas_path.t -> path t Or_error.t

(** Single-line nonempty UTF-8, <=16 KiB, without NUL/CR/LF. The family defaults
    to "system", is nonblank UTF-8 without NUL, and <=128 bytes. Size defaults
    to 14 logical pixels (4..256); weight defaults to 400 (100..900). Native
    shaping/fallback determines actual glyph metrics. *)
val text
  :  id:Id.t
  -> ?generation:int64
  -> ?font_family:string
  -> ?font_size:float
  -> ?font_weight:int
  -> string
  -> text t Or_error.t

(** Borrows an application's registered asset identity. The handle does not
    extend registration lifetime; scene publication acquires its own native lease.
    The Eio adapter checks application ownership before transport. *)
val image : id:Id.t -> ?generation:int64 -> Asset.Handle.t -> image t Or_error.t

module Expert : sig
  type packed

  val pack : _ t -> packed
  val equal_packed : packed -> packed -> bool
  val key : packed -> Gpuio_protocol.Canvas_scene_wire.Resource_key.t

  (** Internal schema conversion. Ownership must be checked before sending the
      resulting value; [Canvas_scene.Expert.encode] performs that check. *)
  val to_wire : packed -> Gpuio_protocol.Canvas_scene_wire.Resource.t

  val belongs_to : packed -> asset_owner:Asset.Expert.Owner.t -> bool
  val path_is_closed : path t -> bool
  val path_corners : path t -> Canvas_geometry.Point.t list
  val path_commands : packed -> int
  val text_bytes : packed -> int
end
