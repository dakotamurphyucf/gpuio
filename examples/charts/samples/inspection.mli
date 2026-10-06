open Core

(** Public chart-inspection presets. Pure configuration; the mounted gallery
    owns state, scoped data registration and event delivery. *)
type t =
  | Default
  | Vertical
  | Band
  | Anchored
  | Cursor
  | Marker_only
[@@deriving equal]

val all : t list
val label : t -> string
val config : t -> Gpuio.Chart_inspection.t
