(** Window-local review progress. The public, separately packaged native counter
    handles activation; typed properties, commands and events update this model.
    The computation remains active while its native inspector is unmounted, so
    reopening restores the observed value without retaining native resources. *)
val component
  :  dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
