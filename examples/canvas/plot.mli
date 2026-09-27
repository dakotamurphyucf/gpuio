open Core

(** Deterministic, simulated model-evaluation data. Position is native world space;
    this sample interprets x as latency and inverted y as quality. *)
module Sample : sig
  type t

  val id : t -> Gpuio.Canvas_scene.Item_id.t
  val name : t -> string
  val color : t -> Gpuio.Color.t
  val position : t -> Gpuio.Canvas_geometry.Point.t
end

type t

val create : large:bool -> t
val samples : t -> Sample.t list
val find : t -> Gpuio.Canvas_scene.Item_id.t -> Sample.t option

val move
  :  t
  -> Gpuio.Canvas_scene.Item_id.t
  -> Gpuio.Canvas_geometry.Transform.t
  -> t Or_error.t

val scene : t -> Gpuio.Canvas_scene.t
