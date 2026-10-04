(** Variable-size cards use the public managed list and reactive configuration.
    Data stays outside transient rows; changing axes preserves surviving models. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Gpuio.Scrollbar.t option Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
