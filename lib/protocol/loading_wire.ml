open Core

module Kind = struct
  type t =
    | Skeleton
    | Shimmer
    | Spinner
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { kind : Kind.t
    ; label : string
    ; animated : bool
    ; period_ms : int
    }
  [@@deriving bin_io, equal, sexp_of]
end
