open Core

(** Encoded asset sources. Construction neither decodes the image nor accesses
    files, URLs, fonts or the native runtime. Acquisition and native registration
    belong to the runtime/Eio adapter. *)
module Format : sig
  type t =
    | Png
    | Jpeg
    | Webp
    | Gif
    | Svg
    | Bmp
    | Tiff
    | Ico
    | Pnm
  [@@deriving equal, compare, sexp_of]

  (** Canonical MIME type; no extension or content sniffing. *)
  val mime_type : t -> string
end

module Source : sig
  type t [@@deriving equal, sexp_of]

  (** Encoded data is nonempty and at most 16 MiB. This is independent of the
      per-message transport bound and the native decoded-pixel/cache budgets.
      Diagnostics report format and length, never the encoded contents. *)
  val max_bytes : int

  (** Accept opaque bytes, including NUL and invalid UTF-8. The caller declares
      the format. Malformed content is diagnosed by native decoding, not here.
      Format membership does not promise every optional codec feature. *)
  val of_bytes : format:Format.t -> string -> t Or_error.t

  val format : t -> Format.t
  val bytes : t -> string
  val byte_length : t -> int
end

module Handle : sig
  (** Immutable reference to one encoded registration in one application.
      It holds no encoded bytes, scope, callback or runtime. Equality includes
      application lifetime as well as the native slot/generation. This value is
      neither portable between applications nor suitable for persistence.

      Keeping it does not keep the registration alive. Releasing the registration
      prevents new native bindings; already mounted readers keep their own leases. *)
  type t [@@deriving equal, sexp_of]

  (** Declared source format, not a successful decode result. *)
  val format : t -> Format.t
end

module Expert : sig
  module Owner : sig
    type t

    (** Fresh application identity; deliberate allocation identity. Contains no
        runtime or I/O capability, and must never be serialized. *)
    val create : unit -> t

    val equal : t -> t -> bool
  end

  val handle
    :  owner:Owner.t
    -> id:Gpuio_protocol.Resource_id.t
    -> format:Format.t
    -> Handle.t

  val belongs_to : Handle.t -> owner:Owner.t -> bool
  val native_id : Handle.t -> Gpuio_protocol.Resource_id.t
end
