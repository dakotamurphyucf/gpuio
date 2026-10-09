(** Optional integration acceptance driver, separate from application setup. *)
val start
  :  env:Eio_unix.Stdenv.base
  -> app:Gpuio_eio.App.t
  -> conversations:Gpuio_agent_chat_runtime.Conversation.t list
  -> windows:(Gpuio_eio.App.Window.t * Gpuio_agent_chat_runtime.Workspace.t) list ref
  -> open_window:(int -> unit)
  -> on_pass:(unit -> unit)
  -> unit
