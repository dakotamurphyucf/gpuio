(** Read-only resource snapshots and explicit native window/picker commands.
    Refresh is user-driven; the preview introduces no recurring sampling timer. *)
val component
  :  Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
