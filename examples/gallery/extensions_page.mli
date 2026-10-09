(** Public package example using the statically composed counter backend. Hiding
    retains the native instance; reset replaces it and page departure disposes it.
    The package owns its inner drawing and accessibility implementation. *)
val component
  :  Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
