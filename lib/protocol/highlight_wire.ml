open Core

let max_specs = 16
let max_ranges = 4096
let max_query_bytes = 4096
let max_config_bytes = 262144

module Query = struct
  type t =
    { text : string
    ; case_sensitive : bool
    ; whole_word : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    (not (String.is_empty t.text))
    && String.length t.text <= max_query_bytes
    && Stdlib.String.is_valid_utf_8 t.text
    && not (String.contains t.text '\000')
  ;;
end

module Range = struct
  type t =
    { start_byte : int64
    ; end_byte : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.start_byte >= 0L && t.end_byte > t.start_byte)
end

module Appearance = struct
  type t =
    { color : int64
    ; active_color : int64
    ; radius : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let color c = Int64.(c >= 0L && c <= 0xffff_ffffL) in
    color t.color
    && color t.active_color
    && Float.is_finite t.radius
    && Float.(t.radius >= 0. && t.radius <= 64.)
  ;;
end

module Spec = struct
  type t =
    { query : Query.t option
    ; ranges : Range.t list
    ; appearance : Appearance.t
    ; active_index : int64 option
    ; match_index_offset : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    (Option.is_some t.query || not (List.is_empty t.ranges))
    && Option.for_all t.query ~f:Query.valid
    && List.length t.ranges <= max_ranges
    && List.for_all t.ranges ~f:Range.valid
    && Appearance.valid t.appearance
    && Option.for_all t.active_index ~f:(fun n -> Int64.(n >= 0L))
    && Int64.(t.match_index_offset >= 0L)
  ;;
end

module Config = struct
  type t = Spec.t list [@@deriving bin_io, equal, sexp_of]

  let valid t =
    List.length t <= max_specs
    && List.for_all t ~f:Spec.valid
    && List.sum (module Int) t ~f:(fun spec -> List.length spec.ranges) <= max_ranges
    && bin_size_t t <= max_config_bytes
  ;;
end
