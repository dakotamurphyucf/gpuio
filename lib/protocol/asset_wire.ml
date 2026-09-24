open Core

let max_chunk_bytes = 262_144

module Format = struct
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

module Error = struct
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

module Request = struct
  type t =
    | Begin of Format.t * int64
    | Append of Resource_id.t * int64 * string
    | Finish of Resource_id.t
    | Release of Resource_id.t
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Begun of Resource_id.t
    | Ack
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]
end
