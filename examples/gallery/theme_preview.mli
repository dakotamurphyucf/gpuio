val component
  :  load:(Gpuio.File_path.t -> Gpuio_gallery_model.Theme_profile.t Core.Or_error.t)
  -> selection:Gpuio_gallery_model.Theme_selection.t Bonsai.Cont.Expert.Var.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
