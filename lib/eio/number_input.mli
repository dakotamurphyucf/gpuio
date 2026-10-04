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

(** Resolve a native application-step intent once. Its original editor lifetime
    and revision are checked even after this controller has remounted. A delayed
    value never overwrites intervening native editing or policy changes. *)
val resolve_step
  :  t
  -> Gpuio.Number_input.Step_request.t
  -> Gpuio.Number_input.Step_resolution.t
  -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
       Bonsai.Effect.t

module Step_task : sig
  module Error : sig
    type t =
      | Work_failed of Error.t
      | Resolution_failed of Gpuio.Number_input.Command_error.t
    [@@deriving sexp_of]
  end

  type t

  (** Cancel computation and queued completion; decline the original native
      request once. Cancellation after a proposal was dispatched cannot undo
      a native edit. No completion callback is delivered after cancellation. *)
  val cancel : t -> unit

  val is_finished : t -> bool
end

(** Run application-step work in an Eio child scope. Pass I/O capabilities in
    [f]'s closure; it must not access Bonsai from another domain. Work exceptions
    become [Work_failed] after queueing a guarded decline. Resolution failures
    also decline and report [Resolution_failed]. Scope/limit failures decline
    and return [Error] without starting work. A scope from another application
    is rejected. Ordinary numeric-command saturation cannot block cleanup.

    Cancellation of [scope] or the returned task suppresses late completion.
    Cleanup never invokes [on_result]. Normal completion retires the child scope
    before invoking [on_result], whose exceptions follow the application's usual
    callback-failure policy. The caller must bind [scope] to its desired lifetime;
    merely hiding a view does not cancel unrelated application scopes. *)
val run_step
  :  t
  -> scope:Scope.t
  -> Gpuio.Number_input.Step_request.t
  -> f:(unit -> Gpuio.Number_input.Step_resolution.t)
  -> on_result:
       ((Gpuio.Number_input.Snapshot.t, Step_task.Error.t) Result.t
        -> unit Bonsai.Effect.t)
  -> Step_task.t Or_error.t Bonsai.Effect.t
