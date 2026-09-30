open Core

(** A small editable scene on a light paper surface. Coordinates are world-space
    logical pixels. Shapes retain their identity across accepted native moves. *)
module Item : sig
  type t

  val id : t -> Gpuio.Canvas_scene.Item_id.t
  val name : t -> string
  val position : t -> Gpuio.Canvas_geometry.Point.t
end

type t

val initial : t
val items : t -> Item.t list
val find : t -> Gpuio.Canvas_scene.Item_id.t -> Item.t option

val move
  :  t
  -> Gpuio.Canvas_scene.Item_id.t
  -> Gpuio.Canvas_geometry.Transform.t
  -> t Or_error.t

val scene : t -> Gpuio.Canvas_scene.t
