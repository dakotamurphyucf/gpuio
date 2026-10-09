open Core

(** Statically linked rich-reader profiles. Definitions/codecs do not register Rust
    behavior; compare schemas with [Gpuio_eio.App.document_profile_catalog]. *)
module Schema = Extension.Schema

module Codec = Extension.Codec
module Stage = Gpuio_protocol.Document_profile_wire.Stage
module Error = Gpuio_protocol.Document_profile_wire.Error

module Definition : sig
  type ('properties, 'event) t

  (** Properties at most 64 KiB; events at most 16 KiB. Pure validated codecs. *)
  val create
    :  schema:Schema.t
    -> properties:'properties Codec.t
    -> events:'event Codec.t
    -> ('properties, 'event) t Or_error.t

  val schema : (_, _) t -> Schema.t
end

module Instance : sig
  type 'event t

  (** Positive generation, nondecreasing while the same schema remains installed.
      Increasing it requests a reset. Property updates also retire old callbacks.
      Clear/reinstall gets a new native epoch independently of this generation. *)
  val create
    :  ('properties, 'event) Definition.t
    -> generation:int64
    -> 'properties
    -> 'event t Or_error.t

  val generation : _ t -> int64
  val schema : _ t -> Schema.t
end

module Signal : sig
  type 'a t =
    | Data of 'a
    | Failed of
        { stage : Stage.t
        ; error : Error.t
        }
  [@@deriving sexp_of]
end

module Event : sig
  (** Source provenance belongs to the installed picture. No AST/native handle is
      exposed. Malformed typed payloads become Input/Invalid_event failures. *)
  type 'a t = private
    { source_revision : int64
    ; source_generation : int64
    ; signal : 'a Signal.t
    }
  [@@deriving sexp_of]
end

module Expert : sig
  val to_wire : _ Instance.t -> Gpuio_protocol.Document_profile_wire.Instance.t

  val event
    :  'a Instance.t
    -> Gpuio_protocol.Document_profile_wire.Event.t
    -> 'a Event.t Or_error.t
end
