val component
  :  save_settings:(Gpuio.File_path.t -> string -> unit Core.Or_error.t)
  -> load_theme:(Gpuio.File_path.t -> Gpuio_gallery_model.Theme_profile.t Core.Or_error.t)
  -> theme_selection:Gpuio_gallery_model.Theme_selection.t Bonsai.Cont.Expert.Var.t
  -> searchable:Selectable_preview.t
  -> edit_filters:Edit_filter_preview.t
  -> app:Gpuio_eio.App.t
  -> desktop:Desktop_session.t
  -> motion:Gpuio.Animation.Preference.t Bonsai.Cont.Expert.Var.t
  -> Gpuio_eio.App.Window.t
  -> page:Gpuio_gallery_model.Page.t Bonsai.Cont.t
  -> palette:Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
