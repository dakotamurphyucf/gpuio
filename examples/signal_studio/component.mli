(** Bonsai observes the current snapshot; Ui.view describes its GPUIO layout. *)
val component
  :  Ui.Snapshot.t Bonsai.Cont.Expert.Var.t
  -> Ui.Actions.t
  -> Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
