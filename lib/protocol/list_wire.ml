open Core

let max_logical_rows = 1_000_000
let max_id_runs = 100_000
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
    ; managed : bool
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

module Viewport = struct
  type t =
    { order_revision : int64
    ; visible_first : int64
    ; visible_last : int64
    ; requested : int64 list
    ; pinned : int64 list
    ; anchor : (int64 * float) option
    ; following_tail : bool
    ; at_start : bool
    ; at_end : bool
    ; budget_exhausted : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let validate t =
    let valid_ids ids =
      List.length ids <= max_active_rows
      && List.for_all ids ~f:(fun id -> Int64.(id > 0L))
      && Set.length (Int64.Set.of_list ids) = List.length ids
    in
    if
      Int64.(
        t.order_revision > 0L
        && t.visible_first >= 0L
        && t.visible_last >= t.visible_first
        && t.visible_last <= of_int max_logical_rows)
      && valid_ids t.requested
      && valid_ids t.pinned
      && Set.length (Int64.Set.of_list (t.requested @ t.pinned)) <= max_active_rows
      && Option.for_all t.anchor ~f:(fun (id, offset) ->
        Int64.(id > 0L)
        && Float.is_finite offset
        && Float.(offset >= 0. && offset <= 1_000_000.))
    then Ok ()
    else Or_error.error_string "invalid list viewport observation"
  ;;
end

module Retained = struct
  type t =
    { node : Node_id.t
    ; rows : int64 list
    }
  [@@deriving bin_io, equal, sexp_of]

  let validate_all notices =
    let nodes = List.map notices ~f:(fun t -> t.node) in
    let count = List.sum (module Int) notices ~f:(fun t -> List.length t.rows) in
    if
      List.is_empty notices
      || count > max_active_rows
      || List.contains_dup nodes ~compare:Node_id.compare
      || List.exists notices ~f:(fun t ->
        List.is_empty t.rows
        || List.exists t.rows ~f:(fun id -> Int64.(id <= 0L))
        || List.contains_dup t.rows ~compare:Int64.compare)
    then Or_error.error_string "invalid list retention response"
    else Ok ()
  ;;
end
