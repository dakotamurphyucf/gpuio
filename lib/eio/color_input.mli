open Core

(** One retained native color editor. Rust owns channel values, drafts, selection,
    composition and undo. Use [view] once per controller. [initial] seeds each
    mount once; configuration updates preserve native edits where policy allows. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Color_input.Config.t Bonsai.Cont.t
  -> initial:Gpuio.Color_value.Value.t
  -> ?on_event:(Gpuio.Color_input.Event.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

(** Editable, allowed palette swatches preview their color and hex spelling on
    hover. This native visual preview does not change snapshots, emit edit
    events, move focus, or replace typed/composing drafts. Selecting a swatch
    still follows the ordinary edit policy. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?appearance:Gpuio.Color_input.Appearance.t
  -> t
  -> Gpuio_bonsai.View.t

(** Last accepted observation; absent before mounting. Unplaced controllers may
    retain an old snapshot; commands then fail [Stale_color_input]. Late replies
    cannot replace a new native lifetime or a newer observation. *)
val snapshot : t -> Gpuio.Color_input.Snapshot.t option

val command
  :  t
  -> Gpuio.Color_input.Command.t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Read observes pending platform edits without committing or focusing. *)
val read_snapshot
  :  t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Explicit Set/Reset validates value and guard before retiring the current edit.
    Success clears obsolete native history/composition and emits [Observed],
    preceded by [Cancelled] for an active edit; it never emits [Committed].
    Available while hidden, disabled or read-only. Reset uses the original mounted
    seed, which may no longer fit current policy. Rejected commands preserve edits. *)
val set
  :  t
  -> ?if_revision:Gpuio.Color_input.Revision.t
  -> Gpuio.Color_value.Value.t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

val reset
  :  t
  -> ?if_revision:Gpuio.Color_input.Revision.t
  -> unit
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Programmatic [Set Empty]; current configuration must permit an empty value. *)
val clear
  :  t
  -> ?if_revision:Gpuio.Color_input.Revision.t
  -> unit
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Cancel restores the committed color and hue and clears any hover preview.
    With no active edit, the color and revision remain unchanged. *)
val cancel
  :  t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Focus a text field; hidden, disabled, modal-blocked and opaque-only alpha
    fields return [Focus_blocked]. Read-only fields may receive focus. Changing
    fields finishes a valid old draft or cancels an invalid/composing old draft.
    In tabbed presentation, an allowed channel focus reveals the channel panel;
    rejected focus leaves the selected panel unchanged. Focus interrupts an
    active channel drag. *)
val focus
  :  t
  -> Gpuio.Color_input.Field.t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Guard the exact native lifetime and revision in [expected]. *)
val set_if_unchanged
  :  t
  -> Gpuio.Color_input.Snapshot.t
  -> Gpuio.Color_value.Value.t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t
