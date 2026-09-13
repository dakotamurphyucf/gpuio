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
