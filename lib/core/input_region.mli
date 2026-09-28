open Core

(** General input observation types. The mounted region adapter is under
    implementation; these types do not enable native event subscriptions yet. *)
module Button = Pointer.Button

module Modifiers = Pointer.Modifiers

module Kind : sig
  type t =
    | Click
    | Auxiliary_click
    | Mouse_down
    | Mouse_up
    | Mouse_move
    | Mouse_enter
    | Mouse_leave
    | Mouse_down_outside
    | Key_down
    | Key_up
    | Focus
    | Blur
    | Scroll
  [@@deriving equal, compare, sexp_of]
end

module Phase : sig
  type t =
    | Capture
    | Bubble
  [@@deriving equal, sexp_of]
end

module Policy : sig
  (** Applied synchronously by Rust, never by the asynchronous OCaml callback. *)
  type t =
    | Observe
    | Stop_propagation
    | Prevent_default
    | Prevent_and_stop
  [@@deriving equal, sexp_of]
end

module Subscription : sig
  type t [@@deriving equal, sexp_of]

  (** Default Bubble/Observe. Click, auxiliary click, enter/leave, outside-down and
      focus/blur only accept those defaults: their notification cannot cancel the
      originating dispatch. Raw down/up/move, key and scroll allow either phase
      and all policies. *)
  val create : Kind.t -> ?phase:Phase.t -> ?policy:Policy.t -> unit -> t Or_error.t

  val kind : t -> Kind.t
end

module Focus : sig
  type t =
    | None
    | Click
    | Tab
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Nonblank UTF-8 label, <=4096 bytes, no NUL. One subscription per kind;
      duplicates and empty lists are errors. Canonical ordering makes subscription
      list order irrelevant. Defaults: enabled and not independently focusable.
      Direct Focus/Blur subscriptions require Click or Tab; key subscriptions can
      observe descendants with None. *)
  val create
    :  label:string
    -> ?disabled:bool
    -> ?focus:Focus.t
    -> Subscription.t list
    -> t Or_error.t

  val subscriptions : t -> Subscription.t list
  val is_disabled : t -> bool
end

module Position : sig
  type t = private
    { x : float
    ; y : float
    }
  [@@deriving equal, sexp_of]
end

module Location : sig
  (** Logical pixels, finite but not necessarily inside the region. *)
  type t = private
    { window : Position.t
    ; local : Position.t
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]
end

module Mouse : sig
  type t = private
    { location : Location.t
    ; button : Button.t
    ; click_count : int
    }
  [@@deriving equal, sexp_of]
end

module Motion : sig
  type t = private
    { location : Location.t
    ; pressed_button : Button.t option
    }
  [@@deriving equal, sexp_of]
end

module Key : sig
  (** Native key identity and optional character, each <=256 UTF-8 bytes without
      NUL. These observations are not committed text or IME composition. *)
  type t = private
    { key : string
    ; character : string option
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]
end

module Delta : sig
  type t =
    | Pixels of Position.t
    | Lines of Position.t
  [@@deriving equal, sexp_of]
end

module Touch_phase : sig
  type t =
    | Started
    | Moved
    | Ended
    | Cancelled
  [@@deriving equal, sexp_of]
end

module Scroll : sig
  (** Preserve native units; zero deltas are valid at phase boundaries. *)
  type t = private
    { location : Location.t
    ; delta : Delta.t
    ; phase : Touch_phase.t
    }
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** Pointer clicks require matched down/up; keyboard/accessibility activation
      belongs to semantic buttons/commands. Movement alone may coalesce, without
      crossing any other event or changing modifiers/pressed-button state. *)
  type t = private
    | Click of Mouse.t
    | Auxiliary_click of Mouse.t
    | Mouse_down of Mouse.t
    | Mouse_up of Mouse.t
    | Mouse_move of Motion.t
    | Mouse_enter
    | Mouse_leave
    | Mouse_down_outside of Mouse.t
    | Key_down of Key.t * bool
    | Key_up of Key.t
    | Focus
    | Blur
    | Scroll of Scroll.t
  [@@deriving equal, sexp_of]

  (** The Boolean carried by Key_down is the native repeat/held flag. *)
  val kind : t -> Kind.t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Input_wire.Config.t
  val event_of_wire : Gpuio_protocol.Input_wire.Event.t -> Event.t Or_error.t
end
