open Core

(** Shared private native-editor identity, observations and revisioned commands. *)
type t

type command =
  Gpuio.Text_input.Snapshot.t
  -> Gpuio.Text_input.Command.t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

val create : App.Window.t -> Bonsai.Cont.graph -> t Bonsai.Cont.t

(** Private transport boundary; the command preserves the exact observed lease. *)
val create_with_command : command -> Bonsai.Cont.graph -> t Bonsai.Cont.t

val key : t -> Gpuio.Key.t
val observe : t -> Gpuio.Text_input.Snapshot.t -> unit Bonsai.Effect.t

(** Replies cannot replace a subsequently observed native mount, even when the
    retired editor's revision is numerically greater than the new revision. *)
val observe_reply
  :  t
  -> expected:Gpuio.Text_input.Snapshot.t
  -> Gpuio.Text_input.Snapshot.t
  -> unit Bonsai.Effect.t

val snapshot : t -> Gpuio.Text_input.Snapshot.t option
val search_snapshot : t -> Gpuio.Text_input.Search.Snapshot.t option
val observe_search : t -> Gpuio.Text_input.Search.Snapshot.t -> unit Bonsai.Effect.t

val observe_search_reply
  :  t
  -> expected:Gpuio.Text_input.Snapshot.t
  -> Gpuio.Text_input.Search.Snapshot.t
  -> unit Bonsai.Effect.t

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
