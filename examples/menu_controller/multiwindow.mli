(** Independent menu/editor ownership for the two-window demonstration. *)
val create
  :  name:string
  -> platform:bool
  -> Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
