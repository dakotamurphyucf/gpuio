open Core

(** Immutable canonical UTF-8 display content. No I/O or native resources.
    Appends share prior chunks and copy at most a 16 KiB tail. Edits and explicit
    flattening are O(total bytes). Old snapshots retain only their shared data;
    there is no unbounded revision journal. *)
type t

module Status : sig
  type t =
    | Streaming
    | Complete
    | Cancelled
  [@@deriving equal, sexp_of]
end

val max_bytes : int
val of_string : ?status:Status.t -> string -> t Or_error.t
val empty_stream : unit -> t
val byte_length : t -> int
val status : t -> Status.t

(** Snapshot identity, suitable for reactive cutoffs. Independently constructed
    equal strings are distinct sources. *)
val equal : t -> t -> bool

val append : t -> string -> t Or_error.t

(** Half-open UTF-8 byte interval; endpoints must be scalar boundaries.
    Corrections preserve generation and terminal status. *)
val edit : t -> first:int -> last:int -> text:string -> t Or_error.t

val replace : t -> string -> t Or_error.t

(** Terminal transitions are idempotent but cannot change one terminal status
    to another. Reset is the explicit start of a new generation. *)
val finish : t -> t Or_error.t

val cancel : t -> t Or_error.t
val reset : t -> ?status:Status.t -> string -> t Or_error.t
val to_string : t -> string
val slice : t -> first:int -> last:int -> string Or_error.t

module Handle : sig
  (** Borrowed native resource identity. Holding this does not extend the
      conversation scope or permit use in a different application. *)
  type t [@@deriving equal, sexp_of]
end

module Decoder : sig
  (** Immutable UTF-8 byte-stream decoder with at most three buffered bytes.
      Each input is at most 256 KiB. A rejected chunk leaves the prior decoder
      usable; no replacement characters or silently dropped bytes. *)
  type t

  val empty : t
  val pending_bytes : t -> int
  val feed : t -> string -> (t * string) Or_error.t
  val finish : t -> unit Or_error.t
end

module Expert : sig
  module Owner : sig
    type t

    val create : unit -> t
    val equal : t -> t -> bool
  end

  val handle : owner:Owner.t -> Gpuio_protocol.Resource_id.t -> Handle.t
  val belongs_to : Handle.t -> owner:Owner.t -> bool
  val native_id : Handle.t -> Gpuio_protocol.Resource_id.t
  val same_source : t -> t -> bool
  val same_generation : t -> t -> bool

  (** First changed chunk boundary, or the old byte length for an append.
      Uses persistent-map sharing; returns zero for unrelated generations.
      The runtime sends the new suffix with an expected accepted revision.
      A status-only change returns the complete byte length. *)
  val changed_from : t -> previous:t -> int

  (** Chunks in byte order; each is valid UTF-8 and at most 16 KiB. *)
  val iter_chunks : t -> f:(offset:int -> string -> unit) -> unit
end
