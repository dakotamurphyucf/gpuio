(** Local portrait fixtures registered once per window on explicit request.
    The unavailable fixture is published successfully but fails native decoding,
    exercising the avatar's real fallback. Window scope retires both assets.
    Repeated toggles reuse registrations and do not accumulate source resources. *)
val component
  :  app:Gpuio_eio.App.t
  -> window:Gpuio_eio.App.Window.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
