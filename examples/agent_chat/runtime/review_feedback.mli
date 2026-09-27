(** Window-local feedback for the simulated run. Keep this computation active
    across inspector route changes. Ratings use ordered requests; note snapshots
    seed remounts but never replace a live native draft. Collapsing the note retains
    its native editor, while guidance sections unmount their pure view content.
    Nothing is persisted or sent to an external service. *)
val component
  :  app:Gpuio_eio.App.t
  -> window:Gpuio_eio.App.Window.t
  -> active:bool Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> on_sources:unit Bonsai.Effect.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
