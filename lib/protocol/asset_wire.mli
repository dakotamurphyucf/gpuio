open Core

(** Internal registration wire types. Variant order is part of the format.
    Appends carry raw bin_prot strings, including NUL/non-UTF-8 bytes. *)
val max_chunk_bytes : int

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
  [@@deriving bin_io, equal, compare, sexp_of]
end

module Error : sig
  type t =
    | Closed
    | Invalid_size
    | Resource_limit
    | Stale_handle
    | Not_uploading
    | Invalid_chunk
    | Incomplete
    | Not_ready
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Request : sig
  type t =
    | Begin of Format.t * int64
    | Append of Resource_id.t * int64 * string
    | Finish of Resource_id.t
    | Release of Resource_id.t
  [@@deriving bin_io, equal, sexp_of]
end

module Response : sig
  type t =
    | Begun of Resource_id.t
    | Ack
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]
end
