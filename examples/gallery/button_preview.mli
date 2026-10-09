(** Rich native buttons with per-owner loading and explicit focus policy. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
