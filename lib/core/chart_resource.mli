open Core

(** Borrowed application-owned native dataset identity. Copying a handle does not
    extend the Eio registration's lifetime; mounting/unmounting a chart is separate
    from registering/releasing its data. *)
type t [@@deriving equal, sexp_of]

module Expert : sig
  module Owner : sig
    type t

    val create : unit -> t
  end

  val handle : owner:Owner.t -> Gpuio_protocol.Resource_id.t -> t
  val belongs_to : t -> owner:Owner.t -> bool
  val native_id : t -> Gpuio_protocol.Resource_id.t
end
