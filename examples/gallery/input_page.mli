(** Captured native pointer input and text/custom/file drop observations. Ordinary
    controls provide keyboard alternatives. Only latest notices/counts are kept;
    incoming file metadata never opens or reads files. Departure clears transient
    gesture feedback while retaining the sample width and drop count. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
