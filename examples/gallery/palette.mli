(** Preview appearance is explicit. Sizes scale in logical pixels; this does not
    simulate an OS display scale or replace platform DPI testing. *)
type t

val create
  :  Gpuio_gallery_model.Appearance.t
  -> Gpuio_gallery_model.Appearance.Scale.t
  -> t

val background : t -> Gpuio.Color.t
val surface : t -> Gpuio.Color.t
val foreground : t -> Gpuio.Color.t
val muted : t -> Gpuio.Color.t
val accent : t -> Gpuio.Color.t
val border : t -> Gpuio.Color.t
val appearance : t -> Gpuio.Presentation.Appearance.t
val theme : t -> Gpuio.Theme.t
val size : t -> float -> float
val text : t -> ?size:float -> ?muted:bool -> string -> Gpuio_bonsai.View.t
val card : t -> title:string -> Gpuio_bonsai.View.t list -> Gpuio_bonsai.View.t
val button : t -> ?selected:bool -> string -> unit Bonsai.Effect.t -> Gpuio_bonsai.View.t
