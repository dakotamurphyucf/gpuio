(** In-memory SVG/raster fixtures and an intentionally malformed source. Native
    loading/ready/failure observations stay separate from encoded registration.
    All four assets belong to one page-visit child scope and retire on departure. *)
val component
  :  Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
