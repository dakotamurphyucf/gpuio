open Core

module Button : sig
  type t =
    | Left
    | Right
    | Middle
    | Back
    | Forward
  [@@deriving equal, sexp_of]
end

module Cancel_reason : sig
  type t =
    | Escape
    | Hidden
    | Blocked
    | Disabled
    | Reconfigured
    | Capture_lost
    | Window_inactive
    | Removed
  [@@deriving equal, sexp_of]
end

module Phase : sig
  type t =
    | Started
    | Moved
    | Released
    | Cancelled of Cancel_reason.t
  [@@deriving equal, sexp_of]
end

module Position : sig
  type t =
    { x : float
    ; y : float
    }
  [@@deriving equal, sexp_of]
end

module Modifiers : sig
  type t =
    { shift : bool
    ; control : bool
    ; alt : bool
    ; command : bool
    ; function_ : bool
    }
  [@@deriving equal, sexp_of]
end

module Gesture_id : sig
  type t [@@deriving equal, compare, sexp_of]
end

module Event : sig
  (** Logical pixels. [local_position] uses the region bounds at this native
      sample, so coordinates may lie outside the region. Cancel uses the last
      observed position. IDs distinguish gestures within one window. Consecutive
      Moved samples may coalesce; lifecycle edges never coalesce. *)
  type t = private
    { gesture : Gesture_id.t
    ; phase : Phase.t
    ; button : Button.t
    ; window_position : Position.t
    ; local_position : Position.t
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Label is bounded nonblank UTF-8 without NUL (<=4096 bytes). Defaults: Left,
      enabled, prevent_default=true, stop_propagation=true. These policies run
      synchronously in Rust. A region captures an initiating press until release
      or cancellation. Provide keyboard alternatives with ordinary controls;
      this raw gesture primitive does not invent slider/button semantics. *)
  val create
    :  label:string
    -> ?button:Button.t
    -> ?disabled:bool
    -> ?prevent_default:bool
    -> ?stop_propagation:bool
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Wire.Pointer.Config.t
  val event_of_wire : Gpuio_protocol.Wire.Pointer.Sample.t -> Event.t Or_error.t
end
