open Core

(** A controlled-value popup picker with a Rust-owned color input draft. [value]
    remains application-owned; only successful explicit confirmation calls
    [on_change]. Channel previews and native text commits do not change the
    confirmed value. Every opening has a fresh color input identity. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Color_input.Config.t Bonsai.Cont.t
  -> value:Gpuio.Color_value.Value.t Bonsai.Cont.t
  -> on_change:(Gpuio.Color_value.Value.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val is_open : t -> bool
val draft : t -> Gpuio.Color_input.Snapshot.t option
val error : t -> Gpuio.Color_picker.Error.t option

(** Presentation hint only; [confirm] always revalidates the native read. *)
val can_confirm : t -> bool

val open_popup : t -> unit Bonsai.Effect.t

(** Cancel, Escape and outside dismissal discard the draft. Captured actions from
    an earlier opening cannot close a newer one. *)
val cancel : t -> unit Bonsai.Effect.t

(** Read the native draft, then revalidate the current opening, application value,
    alpha/empty and read-only policy before calling [on_change] and closing.
    Composing/invalid text, active drags and disallowed values cannot confirm. *)
val confirm
  :  t
  -> (Gpuio.Color_value.Value.t, Gpuio.Color_picker.Error.t) Result.t Bonsai.Effect.t

(** Explicit draft commands do not change the application value. *)
val command
  :  t
  -> Gpuio.Color_input.Command.t
  -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
       Bonsai.Effect.t

(** Ordinary popover composition, using existing placement, dismissal and focus
    restoration. [label] is the trigger text; application formatting is separate
    from canonical color values. [apply_label]/[cancel_label] default to English
    and may be localized. Use one view per controller. The popup contains the
    native hex/channel editors. *)
val view
  :  ?style:Gpuio.Style.t
  -> ?trigger_style:Gpuio.Style.t
  -> ?appearance:Gpuio.Color_input.Appearance.t
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
  -> ?appearance:Gpuio.Color_input.Appearance.t
  -> ?apply_label:string
  -> ?cancel_label:string
  -> overlay:Gpuio.Overlay.Config.t
  -> accessible_name:string
  -> trigger:Gpuio_bonsai.View.t
  -> t
  -> Gpuio_bonsai.View.t Or_error.t
