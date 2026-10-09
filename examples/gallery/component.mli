(** Bonsai wiring: observe state, synchronize the native theme, construct effects,
    and pass current values to the stateless Shell.view. *)
val component
  :  save_settings:(Gpuio.File_path.t -> string -> unit Core.Or_error.t)
  -> load_theme:(Gpuio.File_path.t -> Gpuio_gallery_model.Theme_profile.t Core.Or_error.t)
  -> search_palette:(string -> string list)
  -> searchable:Selectable_preview.t
  -> edit_filters:Edit_filter_preview.t
  -> app:Gpuio_eio.App.t
  -> desktop:Desktop_session.t
  -> motion:Gpuio.Animation.Preference.t Bonsai.Cont.Expert.Var.t
  -> open_window:(unit -> unit)
  -> page:Gpuio_gallery_model.Page.t Bonsai.Cont.Expert.Var.t
  -> appearance:Gpuio_gallery_model.Theme_selection.t Bonsai.Cont.Expert.Var.t
  -> scale:Gpuio_gallery_model.Appearance.Scale.t Bonsai.Cont.Expert.Var.t
  -> custom_chrome:bool
  -> window_snapshot:Gpuio.Window.Snapshot.t option Bonsai.Cont.Expert.Var.t
  -> Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
