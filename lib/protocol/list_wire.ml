open Core

let max_logical_rows = 1_000_000
let max_id_runs = 32_768
let max_active_rows = 16_384

module Id_run = struct
  type t =
    { first : int64
    ; count : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Order = struct
  type t =
    { revision : int64
    ; runs : Id_run.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  let validate t =
    let open Or_error.Let_syntax in
    if Int64.(t.revision < 1L) || List.length t.runs > max_id_runs
    then Or_error.error_string "invalid list order revision or run count"
    else (
      let%bind _, intervals =
        List.fold_result t.runs ~init:(0L, []) ~f:(fun (count, intervals) run ->
          if
            Int64.(run.Id_run.first < 1L || run.count < 1L)
            || Int64.(run.count > of_int max_logical_rows - count)
            || Int64.(run.first > max_value - (run.count - 1L))
          then Or_error.error_string "invalid list identity run or logical row count"
          else
            Ok
              ( Int64.(count + run.count)
              , (run.first, Int64.(run.first + (run.count - 1L))) :: intervals ))
      in
      let sorted =
        List.sort intervals ~compare:(fun (a, _) (b, _) -> Int64.compare a b)
      in
      let%map (_ : int64) =
        List.fold_result sorted ~init:0L ~f:(fun previous_last (first, last) ->
          if Int64.(first <= previous_last)
          then Or_error.error_string "list logical identities overlap"
          else Ok last)
      in
      ())
  ;;
end

module Scroll_policy = struct
  type t =
    | Keep_position
    | Follow_tail_when_at_end
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { estimated_height : float
    ; overscan : float
    ; max_active : int64
    ; scroll_policy : Scroll_policy.t
    ; scrollbar : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let validate t =
    if
      Float.is_finite t.estimated_height
      && Float.(t.estimated_height >= 1. && t.estimated_height <= 1_000_000.)
      && Float.is_finite t.overscan
      && Float.(t.overscan >= 0. && t.overscan <= 1_000_000.)
      && Int64.(t.max_active >= 1L && t.max_active <= of_int max_active_rows)
    then Ok ()
    else Or_error.error_string "invalid list estimate, overscan or active-row budget"
  ;;
end

module Row = struct
  type t =
    { id : int64
    ; node : Node_id.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Scroll_target = struct
  type t =
    | Offset of int64 * float
    | Reveal of int64
    | End
  [@@deriving bin_io, equal, sexp_of]
end

module Scroll_request = struct
  type t =
    { serial : int64
    ; target : Scroll_target.t
    }
  [@@deriving bin_io, equal, sexp_of]
end
