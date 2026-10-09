(** Desktop services are application-owned; represented-file metadata belongs to
    the exact window. File selection does not read contents. Open/reveal and
    notification permission are separate, explicit user actions. *)
val component
  :  Gpuio_eio.App.t
  -> Desktop_session.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
