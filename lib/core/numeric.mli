open Core

(** Bounded binary-float arithmetic shared by numeric controls. This module
    owns no native state and does not promise exact decimal arithmetic. *)
module Direction : sig
  type t =
    | Increase
    | Decrease
  [@@deriving equal, sexp_of]
end

module Domain : sig
  type t [@@deriving equal, sexp_of]

  (** Finite closed bounds and positive step. Equal bounds are valid. Nonempty
      spans must be finite, have at most 2^40 step intervals, and resolve the
      step at the bound magnitudes (eight machine epsilons). *)
  val create : min:float -> max:float -> step:float -> t Or_error.t

  val min : t -> float
  val max : t -> float
  val step : t -> float
  val contains : t -> float -> bool

  (** Clamp and choose the nearest min-anchored grid point or maximum endpoint.
      Exact floating-point ties choose the greater value. Nonfinite input fails. *)
  val normalize : t -> float -> float Or_error.t

  (** Normalize, then select the adjacent point in the requested direction.
      Saturates at endpoints. Does not iterate over the size of the domain. *)
  val advance : t -> float -> direction:Direction.t -> float Or_error.t
end

module Draft : sig
  module Error : sig
    type t =
      | Syntax
      | Non_finite
      | Too_long
    [@@deriving equal, sexp_of]
  end

  type t = private
    | Empty
    | Incomplete
    | Invalid of Error.t
    | Valid of float
    | Out_of_range of float
  [@@deriving equal, sexp_of]

  (** ASCII decimal/exponent grammar; surrounding ASCII whitespace allowed.
      Preserves the caller's draft: this classification never edits text.
      More than 4096 bytes is [Invalid Too_long]. *)
  val parse : Domain.t -> string -> t
end

module Expert : sig
  val to_wire : Domain.t -> Gpuio_protocol.Numeric_wire.Domain.t
  val of_wire : Gpuio_protocol.Numeric_wire.Domain.t -> Domain.t Or_error.t
end
