(** A separately scoped chart example. Bonsai owns controls and requested fixture
    state; GPUIO owns source publication, native preparation and selection.
    Resolved source colors change only through an explicit publication. *)
val component
  :  Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
