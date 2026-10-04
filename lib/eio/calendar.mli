open Core

(** One retained native calendar. Rust owns selection, displayed month and
    keyboard cursor. Use [view] once per controller; duplicate placement rejects.
    [initial] and [initial_month] seed each mount once. Mode is immutable until
    remount; ordinary configuration changes retain selection and navigation,
    including historical selections invalidated by new constraints. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Calendar.Config.t Bonsai.Cont.t
  -> initial:Gpuio.Calendar.Selection.t
  -> initial_month:Gpuio.Calendar.Month.t
  -> ?on_event:(Gpuio.Calendar.Event.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

(** [on_viewport_change] observes the exact logical panes asynchronously,
    independently of [snapshot]. It does not start I/O or a polling timer. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?appearance:Gpuio.Calendar.Appearance.t
  -> ?content:unit Bonsai.Effect.t Gpuio.View.Calendar_content.t
  -> ?on_viewport_change:(Gpuio.Calendar.Viewport.t -> unit Bonsai.Effect.t)
  -> t
  -> Gpuio_bonsai.View.t

(** Last accepted observation, absent before mounting. An unplaced retained
    controller may retain an old snapshot; commands then return [Stale_input].
    A late reply cannot replace a new mounted lease or a newer revision. *)
val snapshot : t -> Gpuio.Calendar.Snapshot.t option

val command
  :  t
  -> Gpuio.Calendar.Command.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Read without focusing or changing state, including hidden/disabled calendars. *)
val read_snapshot
  :  t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Hidden, disabled and modal-blocked calendars return [Focus_blocked]. Read-only
    calendars may receive focus. Success confirms native focus. *)
val focus
  :  t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Reveal and focus a civil date without selecting it. Disabled dates are
    discoverable, but the calendar must pass native focus/visibility gates.
    Unsupported dates return [Invalid_value] before queueing. *)
val focus_date
  :  t
  -> Date.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Navigation preserves selection and clamps the existing cursor day to the
    target month. It aligns the first pane to that month when the civil boundary
    permits. It does not acquire focus or select a date. *)
val show_month
  :  t
  -> Gpuio.Calendar.Month.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Rejects offsets outside +/-119987 or movement beyond the civil domain. *)
val move_months
  :  t
  -> months:int
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

val set_presentation
  :  t
  -> Gpuio.Calendar.Presentation.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Explicit replacement must fit the mounted mode and current constraints.
    Available when disabled/read-only/hidden. Emits [Observed], never [Selected].
    A revision guard rejects rather than overwriting a more recent native change. *)
val replace
  :  t
  -> ?if_revision:Gpuio.Calendar.Revision.t
  -> Gpuio.Calendar.Selection.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

val clear
  :  t
  -> ?if_revision:Gpuio.Calendar.Revision.t
  -> unit
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Fence both the exact native lifetime and revision in [expected]. A remounted
    replacement cannot receive the command even if its revision happens to match. *)
val replace_if_unchanged
  :  t
  -> Gpuio.Calendar.Snapshot.t
  -> Gpuio.Calendar.Selection.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t
