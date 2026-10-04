(** One public description shared by all collection previews; it owns no offsets. *)
type t

val controls : t -> Gpuio_bonsai.View.t
val viewport : t -> Gpuio_bonsai.View.t
val description : t -> Gpuio.Scrollbar.t option

val component
  :  Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t
