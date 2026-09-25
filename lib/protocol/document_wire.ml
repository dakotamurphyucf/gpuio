open Core

let max_bytes = 8 * 1024 * 1024
let max_chunk_bytes = 262_144

module Status = struct
  type t =
    | Streaming
    | Complete
    | Cancelled
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Closed
    | Resource_limit
    | Stale_handle
    | Invalid_revision
    | Invalid_range
    | Invalid_utf8
    | Incomplete
    | Busy
    | Not_ready
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Update = struct
  type t =
    { id : Resource_id.t
    ; base : int64
    ; revision : int64
    ; generation : int64
    ; from_byte : int64
    ; suffix_bytes : int64
    ; status : Status.t
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

module Mode = struct
  type t =
    | Markdown
    | Code of string
    | Diff
  [@@deriving bin_io, equal, sexp_of]
end

module Layout = struct
  type t =
    | Flow
    | Viewport of float
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { source : Resource_id.t option
    ; mode : Mode.t
    ; dark : bool
    ; layout : Layout.t
    ; label : string
    ; path : string option
    ; line_numbers : bool
    ; initially_collapsed : bool
    ; search : string
    ; images : (string * Image_wire.Source.t) list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Side = struct
  type t =
    | Before
    | After
  [@@deriving bin_io, equal, sexp_of]
end

module Navigation = struct
  type t =
    | Link of string
    | Line of string option * Side.t * int64
  [@@deriving bin_io, equal, sexp_of]
end
