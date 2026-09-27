open Core

(** A controlled-value popup picker with a Rust-owned calendar draft. [value]
    remains application-owned; only successful explicit confirmation calls
    [on_change]. Merely selecting a day or completing a range does not commit.
    Every opening has a fresh calendar identity. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Calendar.Config.t Bonsai.Cont.t
  -> value:Gpuio.Calendar.Selection.t Bonsai.Cont.t
  -> initial_month:Gpuio.Calendar.Month.t
  -> on_change:(Gpuio.Calendar.Selection.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val is_open : t -> bool
val draft : t -> Gpuio.Calendar.Snapshot.t option
val error : t -> Gpuio.Date_picker.Error.t option

(** Presentation hint only; [confirm] always revalidates the native read. *)
val can_confirm : t -> bool

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

(** Ordinary popover composition, using existing placement, dismissal and focus
    restoration. [label] is the trigger text; application formatting is separate
    from canonical dates. [apply_label]/[cancel_label] default to English and may
    be localized. Use one view per controller. No editable date field is implied. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?apply_label:string
  -> ?cancel_label:string
  -> overlay:Gpuio.Overlay.Config.t
  -> label:string
  -> t
  -> Gpuio_bonsai.View.t
