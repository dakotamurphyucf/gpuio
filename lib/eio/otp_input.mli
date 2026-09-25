open Core

(** One native OTP editor controlled by explicit commands. Rust owns accepted
    text, preedit, selection and history. Use [view] once per controller; duplicate
    placement rejects. Policy is immutable during a placement. Remount deliberately
    to change its length/alphabet. [initial] seeds each mount once; observations
    and rerenders never overwrite native text. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Otp_input.Config.t Bonsai.Cont.t
  -> initial:Gpuio.Otp_input.Value.t
  -> ?on_event:(Gpuio.Otp_input.Event.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val view : ?style:Gpuio.Style.t -> t -> Gpuio_bonsai.View.t

(** Last accepted observation, absent before mounting. An unplaced retained
    controller may retain an old snapshot; native commands then return
    [Stale_input]. A late reply cannot replace a different mounted lease or a
    newer revision. Snapshots contain actual text even when the field is masked. *)
val snapshot : t -> Gpuio.Otp_input.Snapshot.t option

val command
  :  t
  -> Gpuio.Otp_input.Command.t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Read without focusing or changing state, including hidden/disabled/composing
    fields. No observation is generated for an unchanged state. *)
val read_snapshot
  :  t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Hidden, disabled and modal-blocked fields return [Focus_blocked]. Read-only
    fields may receive focus. Success reflects actual native focus. *)
val focus
  :  t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Directional selection in UTF-8 bytes; rejects composition and invalid offsets. *)
val select
  :  t
  -> Gpuio.Text_input.Selection.t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Explicit canonical replacement. The value must fit the mounted policy even
    if it was constructed under a different policy. Permitted when disabled or
    read-only, but rejects active composition and stale revision guards.
    [Preserve] clamps endpoints; explicit [Select] must fit. [Reset] clears both
    history stacks, including on same-value replacements. Emits no [Complete]. *)
val replace
  :  t
  -> ?if_revision:Gpuio.Otp_input.Revision.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> Gpuio.Otp_input.Value.t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** As replacement with an empty value and caret at zero. *)
val clear
  :  t
  -> ?if_revision:Gpuio.Otp_input.Revision.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> unit
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Fence both the exact native lifetime and revision from [expected]. A later
    remount cannot receive the command even if its revision happens to match. *)
val replace_if_unchanged
  :  t
  -> Gpuio.Otp_input.Snapshot.t
  -> selection:Gpuio.Text_input.Selection_policy.t
  -> undo:Gpuio.Text_input.Undo_policy.t
  -> Gpuio.Otp_input.Value.t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Restore the accepted preedit checkpoint, including after input was disabled
    or hidden. Does not create an undo entry. *)
val cancel_composition
  :  t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Commands restore accepted value/selection and emit [Observed], never user
    [Complete]. Reject composition, disabled and read-only fields. *)
val undo
  :  t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t

val redo
  :  t
  -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
       Bonsai.Effect.t
