open Core

(** A deterministic, explicitly simulated run. Coordinates are canvas world-space
    logical pixels. Moving a stage changes only this window's diagram. *)
module Stage : sig
  type t =
    | Read
    | Draft
    | Review
  [@@deriving equal, sexp_of]

  val all : t list
  val name : t -> string
  val description : t -> string
  val next : t -> t
  val id : t -> Gpuio.Canvas_scene.Item_id.t
  val of_id : Gpuio.Canvas_scene.Item_id.t -> t option
end

type t

val create : unit -> t
val position : t -> Stage.t -> Gpuio.Canvas_geometry.Point.t

(** Only translation is meaningful for these stage labels. Keep a 1,000-pixel
    margin inside the canvas coordinate domain for connectors and arrowheads. *)
val move : t -> Stage.t -> Gpuio.Canvas_geometry.Transform.t -> t Or_error.t

(** [generation] must be positive and increase whenever scene resource contents change. *)
val scene
  :  ?annotation:Gpuio.Color_value.Value.t
  -> t
  -> palette:Palette.t
  -> generation:int64
  -> Gpuio.Canvas_scene.t
