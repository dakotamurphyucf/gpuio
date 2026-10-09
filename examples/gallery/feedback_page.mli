(** Commands share one registry across buttons, menus, shortcuts and the chooser.
    Progress and the bounded toast preview belong to this mounted page. *)
val component
  :  search_palette:(string -> string list)
  -> Gpuio_eio.App.t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
