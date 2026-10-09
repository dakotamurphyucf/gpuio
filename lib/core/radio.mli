open Core

(** Typed semantic values for [View.radio] and [View.radio_with_label]. *)
module Position : sig
  type t [@@deriving equal, sexp_of]

  (** Zero-based [index] in a set of [count] items, with [count] in 1..100,000.
      Disabled items count as members. Native accessibility converts the index
      to one-based position; the application retains responsibility for membership. *)
  val create : index:int -> count:int -> t Or_error.t

  val index : t -> int
  val count : t -> int

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Checkable_wire.Position.t
    val of_wire : Gpuio_protocol.Checkable_wire.Position.t -> t Or_error.t
  end
end
