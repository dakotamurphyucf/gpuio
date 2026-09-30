(** Public subtree-search example. Documents and the query editor belong to this
    page activation; match observations are paired with their configuration. *)
val component
  :  Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
