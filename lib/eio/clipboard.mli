open Core
module Text = Gpuio.Clipboard.Text
module Error = Gpuio.Clipboard.Error

(** Request a plain-text write on the native UI thread. This is an explicit
    application operation, independent of editor/document/table selection.
    Requires the application's UI domain, but no desktop identity or window.

    [Ok ()] means the native GPUI clipboard API was invoked. That API supplies no
    OS persistence/recipient-delivery receipt; another application can replace
    the clipboard immediately. No read, selection clipboard, image or custom
    format operation is implied. Linux native behavior remains experimentally
    supported pending X11/Wayland qualification.

    Uses the bounded shared desktop request lane (16 pending); shutdown resolves
    pending effects with [Closed]. A write already submitted cannot be undone by
    unmounting a caller. Components must fence obsolete feedback themselves.
    Native dispatch never calls OCaml synchronously. *)
val write_text : App.t -> Text.t -> (unit, Error.t) Result.t Bonsai.Effect.t

module Copy : sig
  (** Current-value copy with bounded asynchronous feedback. Busy and the two-second
      copied state suppress duplicate activations. A text/disabled change or Bonsai
      deactivation revokes old feedback and callbacks; an already-submitted OS write
      cannot be undone. Current text is read when the action is reduced, not in a
      synchronous Rust callback. The copied deadline uses Bonsai's shared clock;
      there is no per-component polling loop. *)
  type t

  val create
    :  App.t
    -> text:Text.t Bonsai.Cont.t
    -> ?disabled:bool Bonsai.Cont.t
    -> ?on_copied:(Text.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
    -> Bonsai.Cont.graph
    -> t Bonsai.Cont.t

  val is_busy : t -> bool
  val is_copied : t -> bool
  val error : t -> Error.t option
  val copy : t -> unit Bonsai.Effect.t

  (** Ordinary button composition, with native focus, keyboard and accessibility
      behavior. The default labels are Copy/Copied; callers can use the state and
      [copy] action to compose an icon button or tooltip instead. *)
  val view
    :  t
    -> ?label:string
    -> ?copied_label:string
    -> ?style:Gpuio.Style.t
    -> unit
    -> unit Bonsai.Effect.t Gpuio.View.t
end
