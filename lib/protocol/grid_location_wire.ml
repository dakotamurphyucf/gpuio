open Core

module Edge = struct
  type t =
    | Auto
    | Line of int64
    | Span of int64
  [@@deriving bin_io, equal, sexp_of]
end

module Axis = struct
  type t =
    { start : Edge.t
    ; end_ : Edge.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { column : Axis.t
  ; row : Axis.t
  }
[@@deriving bin_io, equal, sexp_of]
