type t

(** Prepare immutable demo rules once from the application's Eio initialization,
    before opening windows. Never compile during Bonsai evaluation. *)
val prepare : unit -> t Core.Or_error.t

val component
  :  t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
