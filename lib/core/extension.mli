open Core

(** Typed schema and bounded codec building blocks for statically linked native
    components. Creating a schema or codec does not register native behavior. *)
module Schema : sig
  type t [@@deriving equal, sexp_of]

  (** Qualified name: 2 or more dot-separated ASCII segments, each beginning
      with [a-z] and continuing with [a-z0-9_], at most 128 bytes total.
      Version is 1..65535. Fingerprint is exactly 64 lowercase hexadecimal
      characters, supplied by the author for the complete wire schema. *)
  val create : name:string -> version:int -> fingerprint:string -> t Or_error.t

  val name : t -> string
  val version : t -> int
  val fingerprint : t -> string

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Extension_wire.Schema.t
    val of_wire : Gpuio_protocol.Extension_wire.Schema.t -> t Or_error.t
  end
end

module Codec : sig
  type 'a t

  (** [max_bytes] is 1..65536. Callbacks are pure trusted package code, run on the
      OCaml UI domain. Expected failures use [Or_error]; raised exceptions are
      contained as errors. [decode] must consume its complete input and enforce
      domain invariants. Neither callback may perform I/O. *)
  val create
    :  max_bytes:int
    -> encode:('a -> string Or_error.t)
    -> decode:(string -> 'a Or_error.t)
    -> 'a t Or_error.t

  (** Bin_prot adapter checks size before allocating, enforces exact read/write
      consumption and validates values on both encode and decode. *)
  val bin_prot
    :  max_bytes:int
    -> 'a Bin_prot.Type_class.t
    -> validate:('a -> unit Or_error.t)
    -> 'a t Or_error.t

  val max_bytes : 'a t -> int
  val encode : 'a t -> 'a -> string Or_error.t
  val decode : 'a t -> string -> 'a Or_error.t
end

module Error = Gpuio_protocol.Extension_wire.Error

module Event : sig
  type 'a t =
    | Data of 'a
    | Mounted
    | Command_completed of int64
    | Failed of Error.t
  [@@deriving sexp_of]
end

module Definition : sig
  type ('properties, 'command, 'event) t

  (** Properties may use at most64KiB; command and event codecs at most16KiB.
      The schema covers all three codecs. Construction is pure and does not
      assert that a matching native factory is linked into this application. *)
  val create
    :  schema:Schema.t
    -> properties:'properties Codec.t
    -> commands:'command Codec.t
    -> events:'event Codec.t
    -> ('properties, 'command, 'event) t Or_error.t

  val schema : (_, _, _) t -> Schema.t
end

module Command : sig
  type 'a t

  (** Positive, monotonically increasing per mounted instance. Native rendering
      never repeats a command with the same sequence. Reusing a sequence with
      different bytes is invalid. New instance generations start a new sequence. *)
  val create : sequence:int64 -> 'a -> 'a t Or_error.t
end

module Instance : sig
  type 'event t

  (** [generation] is positive and must not decrease for a retained node. Increase
      it to dispose/reset native state. Ordinary property changes update the
      existing component and invalidate callbacks from obsolete properties.
      [disabled] suppresses native input/events, not explicit command delivery;
      the component's documented command policy decides which commands can run. *)
  val create
    :  ('properties, 'command, 'event) Definition.t
    -> generation:int64
    -> label:string
    -> ?disabled:bool
    -> ?command:'command Command.t
    -> 'properties
    -> 'event t Or_error.t

  module Expert : sig
    val to_wire : _ t -> Gpuio_protocol.Extension_wire.Config.t
    val event : 'a t -> Gpuio_protocol.Extension_wire.Signal.t -> 'a Event.t
  end
end
