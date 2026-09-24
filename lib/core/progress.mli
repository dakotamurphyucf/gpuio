open Core

module Value : sig
  type t [@@deriving equal, sexp_of]

  val indeterminate : t

  (** A finite completed fraction in [0, 1]. Invalid values are rejected rather
      than clamped. *)
  val determinate : fraction:float -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Accessible label: nonblank UTF-8 without NUL, at most 4096 bytes. *)
  val create : label:string -> value:Value.t -> t Or_error.t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Wire.Progress.t
end
