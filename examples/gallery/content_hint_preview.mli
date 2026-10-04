(** Hint configuration and asynchronous native exposure status, using synthetic
    contact text. The displayed observation includes the hint at query time. *)
val component
  :  Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
