open Core

(** Public Settings composite example. All data/editor controllers and save tasks
    live outside transient rows. The supplied writer runs inside a window-scoped
    Eio task and receives a native user-selected destination and immutable bytes. *)
val component
  :  save:(Gpuio.File_path.t -> string -> unit Or_error.t)
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
