(** Public style composition for semantic button variants, sizing and tooltips.
    Appearance selection is independent of an action's toggled semantic value. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
