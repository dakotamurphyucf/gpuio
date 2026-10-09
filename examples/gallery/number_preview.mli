(** Retained numeric drafts, commit/cancel and all native stepper arrangements. *)
val component
  :  Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> read_only:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
