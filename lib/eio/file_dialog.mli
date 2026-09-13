open Core
module Open = Gpuio.File_dialog.Open
module Save = Gpuio.File_dialog.Save
module Error = Gpuio.File_dialog.Error

(** Present a native picker after the window has opened. One pending picker per
    window; overlap returns [Busy], pre-open calls [Not_ready]. User cancellation
    returns [Ok None]; window closure/application shutdown returns [Error Closed].
    Paths are absolute native bytes, not filesystem capabilities. Use Eio for
    subsequent file I/O. Native backend/platform differences return [Unsupported]. *)
val open_
  :  App.Window.t
  -> config:Open.t
  -> (Gpuio.File_path.t list option, Error.t) Result.t Bonsai.Effect.t

(** Select one save destination without creating, reserving or writing it. *)
val save
  :  App.Window.t
  -> config:Save.t
  -> (Gpuio.File_path.t option, Error.t) Result.t Bonsai.Effect.t
