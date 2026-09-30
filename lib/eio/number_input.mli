open Core

(** A Bonsai controller for one native numeric editor. Rust owns text, selection,
    composition and undo history. Use [view] once; duplicate placement is rejected.
    Observations never replace text. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Number_input.Config.t Bonsai.Cont.t
  -> initial:Gpuio.Number_input.Value.t
  -> ?on_event:(Gpuio.Number_input.Event.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

(** Mount-only seeds: [initial] overrides the committed value; [initial_draft]
    supplies independent text, including an unfinished/invalid expression.
    Omitting the draft formats the normalized committed value. Neither seed
    replaces a live draft. Selection, IME and undo history start fresh.

    [on_event] observes native events for this placement, alongside the
    controller's normal observation and [create] event handlers. It does not run for
    command replies: handle those results explicitly. Use the row lifetime to
    guard this callback when mirroring application drafts from transient rows;
    a retained controller's last snapshot can outlive its native placement. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?initial:Gpuio.Number_input.Value.t
  -> ?initial_draft:Gpuio.Number_input.Draft.t
  -> ?on_event:(Gpuio.Number_input.Event.t -> unit Bonsai.Effect.t)
  -> t
  -> Gpuio_bonsai.View.t

(** Last native observation; absent before mounting. A retained controller can
    outlive its placement, in which case commands fail with [Stale_input]. *)
val snapshot : t -> Gpuio.Number_input.Snapshot.t option

val command
  :  t
  -> Gpuio.Number_input.Command.t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Reads current state without editing or focusing, including hidden, disabled
    or composing fields. *)
val read_snapshot
  :  t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Returns [Focus_blocked] when the native field cannot receive focus, including
    disabled, hidden or modal-blocked placement. Read-only fields can be focused. *)
val focus
  :  t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

val select
  :  t
  -> Gpuio.Text_input.Selection.t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Explicit replacements, permitted while disabled/read-only. Both reject active
    composition and honor numeric revision guards atomically. Selection offsets
    are UTF-8 bytes. Replacing a draft leaves the committed value unchanged;
    replacing a value normalizes it and replaces both committed value and text. *)
val replace_draft
  :  t
  -> ?if_revision:Gpuio.Number_input.Revision.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> string
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

val replace_value
  :  t
  -> ?if_revision:Gpuio.Number_input.Revision.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> Gpuio.Number_input.Value.t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Fences both the native lifetime and numeric revision in [expected]. A later
    remount cannot receive the replacement, even if its revision matches. *)
val replace_value_if_unchanged
  :  t
  -> Gpuio.Number_input.Snapshot.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> Gpuio.Number_input.Value.t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Normalize a finite draft, including clamping to bounds. Rejected drafts and
    active composition retain text and selection. Losing focus does not commit. *)
val commit
  :  t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Restore committed text. Does not discard active composition. *)
val cancel
  :  t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Step and commit; rejects disabled/read-only, composing or invalid/incomplete
    drafts. An empty draft seeds normalized zero without an additional step. *)
val step
  :  t
  -> Gpuio.Numeric.Direction.t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Undo/redo change the draft and selection, not the committed value. *)
val undo
  :  t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

val redo
  :  t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t
