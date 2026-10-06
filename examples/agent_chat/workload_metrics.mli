(** Opt-in diagnostic sampling. Not part of normal application startup. *)
val start
  :  env:Eio_unix.Stdenv.base
  -> app:Gpuio_eio.App.t
  -> conversations:Gpuio_agent_chat_runtime.Conversation.t list
  -> unit
