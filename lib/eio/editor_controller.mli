open Core

(** Shared private native-editor identity, observations and revisioned commands. *)
type t

val create : App.Window.t -> Bonsai.Cont.graph -> t Bonsai.Cont.t
val key : t -> Gpuio.Key.t
val observe : t -> Gpuio.Text_input.Snapshot.t -> unit Bonsai.Effect.t
val snapshot : t -> Gpuio.Text_input.Snapshot.t option

val command
  :  t
  -> Gpuio.Text_input.Command.t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val focus
  :  t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val select
  :  t
  -> Gpuio.Text_input.Selection.t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val replace
  :  t
  -> ?if_revision:Gpuio.Text_input.Revision.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> string
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val replace_if_unchanged
  :  t
  -> Gpuio.Text_input.Snapshot.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> string
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t
