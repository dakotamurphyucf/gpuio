(** Public controller, registry invocation and stale-definition demonstration. *)
val create
  :  platform:bool
  -> app:Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
