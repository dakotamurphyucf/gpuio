open Core

(** Deterministic model-evaluation workspace. Only this module knows fixture
    identities and world-space bounds; no I/O or native resources are owned here. *)
module Sample : sig
  type t

  val id : t -> Gpuio.Canvas_scene.Item_id.t
  val name : t -> string
  val color : t -> Gpuio.Color.t

  (** Axis units: latency in milliseconds, quality in percent. *)
  val latency : t -> float

  val quality : t -> float
  val position : t -> Gpuio.Canvas_geometry.Point.t
end

type t

val create : unit -> t
val samples : t -> Sample.t list
val selected : t -> Sample.t option
val run : t -> int
val set_run : t -> int -> t Or_error.t
val select : t -> Gpuio.Canvas_scene.Item_id.t option -> t Or_error.t

(** Unknown IDs fail. Native world-space motion is clamped to the plot's bounds;
    rotations/scales are not persisted as movement of a data point. *)
val move
  :  t
  -> Gpuio.Canvas_scene.Item_id.t
  -> Gpuio.Canvas_geometry.Point.t
  -> t Or_error.t

val scene : t -> Gpuio.Canvas_scene.t
val chart : t -> Gpuio.Chart_data.t

(** Version-1 sexp document, <=16 KiB. Decoding validates the version, run 0..100,
    all four unique fixture identities, finite in-bounds positions and selection.
    Scene and chart handles are regenerated, never serialized. *)
val encode : t -> string

val decode : string -> t Or_error.t

(** Only gpuio-signal://sample/{1..4} with no query/fragment is a route. Parsing
    never opens a resource or performs I/O. *)
val scheme : Gpuio.Deep_link.Scheme.t

val route : t -> Gpuio.Deep_link.t -> t Or_error.t
