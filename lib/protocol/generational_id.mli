open Core

exception Invalid_wire_handle

module type S = sig
  type t [@@deriving bin_io, compare, equal, sexp_of]

  (** Slots are 0 through 2^32-1; generations are 1 through 2^32-1.
      Decoding validates the same invariant. Generations must never wrap. *)
  val create : slot:int64 -> generation:int64 -> t Or_error.t

  val slot : t -> int64
  val generation : t -> int64
end

(** Each application creates a distinct abstract identity type. *)
module Make () : S
