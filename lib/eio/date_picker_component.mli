open Core

(** A controlled-value popup picker with a Rust-owned calendar draft. [value]
    remains application-owned; only successful explicit confirmation calls
    [on_change]. Merely selecting a day or completing a range does not commit.
    Every opening has a fresh calendar identity. *)
type command =
  Gpuio.Calendar.Snapshot.t
  -> Gpuio.Calendar.Command.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

type t

val create
  :  command
  -> config:Gpuio.Calendar.Config.t Bonsai.Cont.t
  -> value:Gpuio.Calendar.Selection.t Bonsai.Cont.t
  -> initial_month:Gpuio.Calendar.Month.t
  -> on_change:(Gpuio.Calendar.Selection.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val is_open : t -> bool

(** Apply and other preset requests are unavailable while a preset command is
    awaiting its native reply. Ordinary draft commands remain explicit operations. *)
val is_selecting_preset : t -> bool

(** [None] while the native calendar is mounting, even when [is_open] is true.
    Opening the popup or receiving a window frame acknowledgment does not wait
    for this asynchronous observation. Read the current reactive value when
    testing readiness; a captured older [t] does not acquire later snapshots. *)
val draft : t -> Gpuio.Calendar.Snapshot.t option

val error : t -> Gpuio.Date_picker.Error.t option

(** Presentation hint only; [confirm] always revalidates the native read. *)
val can_confirm : t -> bool

(** Request an opening. Completion means the model action was injected, not that
    the native draft is ready; observe [draft] on the current reactive value. *)
val open_popup : t -> unit Bonsai.Effect.t

(** Cancel, Escape and outside dismissal discard the draft. Captured actions from
    an earlier opening cannot close a newer one. *)
val cancel : t -> unit Bonsai.Effect.t

(** Read the native draft, then revalidate the current opening, application value,
    constraints and read-only policy before calling [on_change] and closing.
    Empty/complete selections may confirm; a partial range cannot. *)
val confirm
  :  t
  -> (Gpuio.Calendar.Selection.t, Gpuio.Date_picker.Error.t) Result.t Bonsai.Effect.t

(** Explicit draft commands do not change the application value. *)
val command
  :  t
  -> Gpuio.Calendar.Command.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t Bonsai.Effect.t

(** Replace the open draft with a complete preset; never calls [on_change] or
    closes the popup. Check the latest configuration and opening before sending
    a revision-guarded native replacement. Delayed replies cannot affect another
    opening. Concurrent requests fail with [Native Busy]; confirmation during a
    pending request also fails with [Native Busy]. Presets do not move the month
    cursor or focus. Use [command] for explicit navigation. *)
val select_preset
  :  t
  -> Gpuio.Date_picker.Preset.t
  -> (Gpuio.Calendar.Snapshot.t, Gpuio.Date_picker.Error.t) Result.t Bonsai.Effect.t

(** Ordinary popover composition, using existing placement, dismissal and focus
    restoration. [label] is the trigger text; application formatting is separate
    from canonical dates. [apply_label]/[cancel_label] default to English and may
    be localized. [appearance] styles the native draft calendar. Use one view per
    controller. No editable date field is implied. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?trigger_style:Gpuio.Style.t
  -> ?appearance:Gpuio.Calendar.Appearance.t
  -> ?calendar_content:unit Bonsai.Effect.t Gpuio.View.Calendar_content.t
  -> ?on_calendar_viewport_change:(Gpuio.Calendar.Viewport.t -> unit Bonsai.Effect.t)
  -> ?presets:Gpuio.Date_picker.Preset.Collection.t
  -> ?apply_label:string
  -> ?cancel_label:string
  -> overlay:Gpuio.Overlay.Config.t
  -> label:string
  -> t
  -> Gpuio_bonsai.View.t

(** A single native trigger button around checked passive content (for example
    formatted value text, an icon or a color swatch). [accessible_name] must be
    nonblank UTF-8 without NUL, at most 1024 bytes. Content follows
    [Gpuio.View.button_with_content]'s bounds; interactive descendants and
    callbacks are rejected. Changing content retains the trigger and open draft.
    [trigger_style] styles the button; [style] styles the popup. Use one view per
    controller, choosing this helper or [view]. A clear action belongs beside
    the trigger and updates the application's controlled value explicitly. *)
val view_with_trigger
  :  ?style:Gpuio.Style.t
  -> ?trigger_style:Gpuio.Style.t
  -> ?appearance:Gpuio.Calendar.Appearance.t
  -> ?calendar_content:unit Bonsai.Effect.t Gpuio.View.Calendar_content.t
  -> ?on_calendar_viewport_change:(Gpuio.Calendar.Viewport.t -> unit Bonsai.Effect.t)
  -> ?presets:Gpuio.Date_picker.Preset.Collection.t
  -> ?apply_label:string
  -> ?cancel_label:string
  -> overlay:Gpuio.Overlay.Config.t
  -> accessible_name:string
  -> trigger:Gpuio_bonsai.View.t
  -> t
  -> Gpuio_bonsai.View.t Or_error.t
