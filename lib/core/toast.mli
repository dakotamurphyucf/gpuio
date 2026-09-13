open Core

module Timeout : sig
  type t [@@deriving equal, sexp_of]

  val persistent : t

  (** Positive active time, at most 24 hours. Hidden or blocked notifications and
      stacks containing pointer hover or keyboard focus pause their timers. *)
  val after : Time_ns.Span.t -> t Or_error.t
end

module Politeness : sig
  type t =
    | Polite
    | Assertive
  [@@deriving equal, sexp_of]
end

module Dismissal : sig
  type t =
    | Timeout
    | Close_button
    | Escape
    | Overflow
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults: five seconds of active time, polite announcement and "Dismiss
      notification" as the close-button label. Labels are nonblank UTF-8 without
      NUL; notification label <=4096 bytes, close label <=256 bytes. *)
  val create
    :  label:string
    -> ?timeout:Timeout.t
    -> ?politeness:Politeness.t
    -> ?close_label:string
    -> unit
    -> t Or_error.t
end

module Corner : sig
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving equal, sexp_of]
end

module Stack : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults: label "Notifications", bottom right, width 360 logical pixels,
      three visible notifications. Width must be finite/positive <=1,000,000;
      max_visible is 1..8. Older overflow is dismissed, never retained as an
      unbounded hidden queue. At most 32 keyed toast views may be submitted. *)
  val create
    :  ?label:string
    -> ?corner:Corner.t
    -> ?width:float
    -> ?max_visible:int
    -> unit
    -> t Or_error.t

  val default : t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Wire.Toast.t
  val stack_to_wire : Stack.t -> Gpuio_protocol.Wire.Toast_stack.t

  (** A terminal native dismissal remains valid after an ordinary configuration
      update. The reconciler separately checks the live node and handler. *)
  val dismissal : Gpuio_protocol.Wire.Toast_dismissal.t -> Dismissal.t
end
