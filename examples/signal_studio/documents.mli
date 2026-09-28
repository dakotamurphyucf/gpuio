open Core
module Workspace = Signal_studio_model.Workspace
module App = Gpuio_eio.App

module State : sig
  type t =
    { path : Gpuio.File_path.t option
    ; edited : bool
    ; busy : bool
    ; metadata : (unit, Gpuio.Window.Error.t) Result.t option
    }

  val initial : t
end

module Model : sig
  type t =
    { current : unit -> Workspace.t
    ; replace : Workspace.t -> unit
    ; stop_stream : unit -> unit
    ; report : string -> unit
    }
end

type t

(** UI-domain controller. [load] and [save] run in application-scoped Eio fibers
    and must capture explicit filesystem capabilities. The controller serializes
    pickers/I/O. Save records the submitted snapshot, so later edits remain dirty.
    Load preserves edits made while reading. Reset fences stale completions.
    Native metadata failure is independent of file success. *)
val create
  :  App.t
  -> model:Model.t
  -> window:(unit -> App.Window.t option)
  -> load:(Gpuio.File_path.t -> Workspace.t Or_error.t)
  -> save:(Gpuio.File_path.t -> Workspace.t -> unit Or_error.t)
  -> on_state:(State.t -> unit)
  -> t

val state : t -> State.t
val refresh : t -> unit Bonsai.Effect.t
val reset : t -> unit Bonsai.Effect.t
val open_ : t -> directory:Gpuio.File_path.t -> unit Bonsai.Effect.t
val save_as : t -> directory:Gpuio.File_path.t -> unit Bonsai.Effect.t
val reveal : t -> unit Bonsai.Effect.t

(** Explicit destinations, used by the integration test. These never interpret a
    deep link as a file path. Caller is responsible for destination authorization. *)
val load_path : t -> Gpuio.File_path.t -> unit Bonsai.Effect.t

val save_path : t -> Gpuio.File_path.t -> unit Bonsai.Effect.t
