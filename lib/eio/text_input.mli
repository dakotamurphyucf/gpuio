open Core

(** A Bonsai controller for one native editor placement. Rust owns the editing
    session. Use [view] once in a window tree; duplicate placement is rejected.
    An observation is never an implicit text replacement. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Text_input.Config.t Bonsai.Cont.t
  -> ?initial_text:string
  -> ?on_submit:(Gpuio.Text_input.Submission.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val view : ?style:Gpuio.Style.t -> t -> Gpuio_bonsai.View.t

(** Last native observation, absent before the first mount. A stored controller
    may refer to an unmounted lease; commands then return [Stale_editor]. *)
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

(** Clear exactly the lease/revision submitted. Later typing, including typing
    and deleting back to the same text, causes [Stale_revision]. A replacement
    editor causes [Stale_editor]. The clear is recorded in native undo history. *)
val clear_if_unchanged
  :  t
  -> Gpuio.Text_input.Submission.t
  -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Ask the native editor for an exact submission snapshot, then invoke the
    [on_submit] handler supplied to [create]. Unlike [snapshot], this waits for
    the native command and cannot submit an older Bonsai observation. Active
    composition and unavailable/hidden editors fail without invoking the handler.
    Success means the handler completed; it does not imply an external send
    was accepted. Use [clear_if_unchanged] after application acceptance. *)
val submit : t -> (unit, Gpuio.Text_input.Command_error.t) Result.t Bonsai.Effect.t
