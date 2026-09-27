open Core

(** Application-scoped native desktop integration. Pure configuration only;
    operations and readiness belong to the Eio application runner. *)
module Identity : sig
  type t [@@deriving equal, sexp_of]

  (** [identifier] is a lowercase reverse-DNS style ID, at most 128 bytes,
      with at least two nonempty segments. Each segment starts with a letter
      and contains letters/digits/hyphens, without a trailing hyphen. [name]
      contains 1..256 UTF-8 bytes without ASCII control characters and is not
      blank. At most 16 unique normalized [schemes] are accepted.

      Identity is process-wide and configured once before desktop requests.
      Scheme declarations do not install or reassign OS handlers; packaged
      application metadata must agree with this identity. *)
  val create
    :  identifier:string
    -> name:string
    -> ?schemes:Deep_link.Scheme.t list
    -> unit
    -> t Or_error.t

  val identifier : t -> string
  val name : t -> string
  val schemes : t -> Deep_link.Scheme.t list
end

module Capabilities : sig
  (** Support, not proof of current availability or authorization. An operation
      can still return [Unavailable] or [Denied]. Capability snapshots never
      imply a notification/link reached a user or another application. *)
  type t =
    { incoming_links : bool
    ; runtime_registration : bool
    ; application_activation : bool
    ; file_reveal : bool
    ; file_open : bool
    ; document_metadata : bool
    }
  [@@deriving equal, sexp_of]
end

module Error : sig
  type t =
    | Invalid_request
    | Not_ready
    | Already_configured
    | Unsupported
    | Unavailable
    | Denied
    | Busy
    | Closed
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val identity_to_wire : Identity.t -> Gpuio_protocol.Desktop_wire.Identity.t
  val capabilities_of_wire : Gpuio_protocol.Desktop_wire.Capabilities.t -> Capabilities.t
  val error_of_wire : Gpuio_protocol.Desktop_wire.Error.t -> Error.t
end

module Event : sig
  type t =
    | Link of Deep_link.t
    | Rejected_link of
        { input : string
        ; reason : Deep_link.Error.t
        }
    | Overflow of int64
    | Failed of Error.t
  [@@deriving equal, sexp_of]
end
