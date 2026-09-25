(** Public-API native acceptance fixture for constrained presentation content.
    The toolbar cycles through component families and content variants. *)
val component
  :  Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
