open Core

module Tab_order = struct
  type t =
    { tab_stop : bool
    ; index : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.index >= -1000000L && t.index <= 1000000L)
end

module Position = struct
  type t =
    { index : int64
    ; count : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.index >= 0L && t.index < t.count && t.count <= 100000L)
end
