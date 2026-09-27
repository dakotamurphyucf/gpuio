open Core

(* Internal resource transport; datasets are uploaded separately from Views.
   [generation] changes on a logical reset, while every publication increments
   [revision]. Abort never changes the currently published dataset. *)
let max_chunk_bytes = 262_144

module Error = struct
  type t =
    | Closed
    | Resource_limit
    | Stale_handle
    | Invalid_revision
    | Invalid_range
    | Incomplete
    | Busy
    | Not_ready
    | Invalid_data
    | Cancelled
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Update = struct
  type t =
    { id : Resource_id.t
    ; base : int64
    ; revision : int64
    ; generation : int64
    ; bytes : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Request = struct
  type t =
    | Create
    | Begin of Update.t
    | Chunk of Resource_id.t * int64 * int64 * string
    | Publish of Resource_id.t * int64
    | Abort of Resource_id.t * int64
    | Release of Resource_id.t
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Created of Resource_id.t
    | Ack
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]
end
