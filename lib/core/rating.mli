open Core

module Request : sig
  (** Discrete requests are applied against the application's latest value.
      Increments/decrements are never computed from a stale rendered value.
      Hover preview generates no request. *)
  type t = private
    | Set of int
    | Toggle of int
    | Increase
    | Decrease
  [@@deriving equal, sexp_of]

  val set : int -> t Or_error.t
  val toggle : int -> t Or_error.t
  val clear : t
  val increase : t
  val decrease : t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Maximum is 1..32 (default 5), value is 0..maximum. Zero is unrated.
      Invalid configuration is rejected, never clamped. The accessible label
      requires nonblank UTF-8 without NUL, at most 4096 bytes. [star_size] is
      8..128 logical pixels (default 24). Ordinary styles control the enclosing
      box and foreground; inactive stars use reduced opacity.

      Disabled controls leave keyboard traversal. Read-only controls retain
      focus and value semantics but emit no requests or hover preview. *)
  val create
    :  label:string
    -> value:int
    -> ?maximum:int
    -> ?star_size:float
    -> ?disabled:bool
    -> ?read_only:bool
    -> unit
    -> t Or_error.t

  val value : t -> int
  val maximum : t -> int
  val star_size : t -> float
  val is_disabled : t -> bool
  val is_read_only : t -> bool

  (** Apply inside the application's state-machine reducer, in request order.
      Steps saturate at zero/maximum. Toggle selects its star, or clears when
      that star is already selected. Requests outside the current maximum and
      requests to disabled/read-only configurations are ignored. This allows a
      queued request to be safely reduced after application constraints change.
      This pure function performs no I/O and creates no native controller. *)
  val apply_request : t -> Request.t -> t
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** Independent filled-star and outline colors. Omitted colors use computed
      foreground (inactive: 0.7 opacity). Explicit colors preserve their alpha
      and override foreground state refinements. Theme tokens resolve at View
      submission; ancestor opacity/clipping still apply. *)
  val create : ?active:Color.t -> ?inactive:Color.t -> unit -> t

  val default : t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Rating_wire.Config.t
  val request_of_wire : Gpuio_protocol.Rating_wire.Request.t -> Request.t option
  val can_apply : Config.t -> Request.t -> bool

  val appearance_to_wire
    :  Appearance.t
    -> theme:Theme.t
    -> Gpuio_protocol.Rating_wire.Appearance.t option Or_error.t
end
