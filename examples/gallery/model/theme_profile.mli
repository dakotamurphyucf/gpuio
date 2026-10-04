open Core

(** Validated application example format, independent of native theme globals. *)
type t [@@deriving equal]

val maximum_bytes : int
val decode : string -> t Or_error.t
val name : t -> string
val appearance : t -> Appearance.t
val background : t -> Gpuio.Color.t
val surface : t -> Gpuio.Color.t
val foreground : t -> Gpuio.Color.t
val muted : t -> Gpuio.Color.t
val accent : t -> Gpuio.Color.t
val border : t -> Gpuio.Color.t
val presentation : t -> Gpuio.Presentation.Appearance.t
