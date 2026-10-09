open Core
module Selection = Chart_selection_wire

module Target = struct
  type t =
    | Cartesian of int64 * int64
    | Slice of int64
    | Radar of int64 * int64
    | Candlestick of int64
    | Node of int64
    | Edge of int64
    | Aggregate of
        { source : Resource_id.t
        ; data_revision : int64
        ; data_generation : int64
        ; selection : Selection.t
        }
  [@@deriving bin_io, compare, equal, sexp_of]

  let is_aggregate = function
    | Selection.Cartesian { aggregation = Sum | Mean; _ }
    | Candlestick { aggregated = true; _ } -> true
    | Cartesian { aggregation = Exact; _ }
    | Candlestick { aggregated = false; _ }
    | Slice _ | Radar _ | Node _ | Edge _ -> false
  ;;

  let valid = function
    | Cartesian (series, datum) | Radar (series, datum) ->
      Int64.(series > 0L && datum > 0L)
    | Slice id | Candlestick id | Node id | Edge id -> Int64.(id > 0L)
    | Aggregate { source = _; data_revision; data_generation; selection } ->
      Int64.(data_revision > 0L && data_generation > 0L)
      && Selection.valid selection
      && is_aggregate selection
  ;;

  let of_selection selection ~source ~data_revision ~data_generation =
    if
      Int64.(data_revision <= 0L || data_generation <= 0L)
      || not (Selection.valid selection)
    then None
    else
      Some
        (match selection with
         | Selection.Cartesian { series; span; aggregation = Exact } ->
           Cartesian (series, span.first)
         | Candlestick { span; aggregated = false } -> Candlestick span.first
         | Slice id -> Slice id
         | Radar { series; axis } -> Radar (series, axis)
         | Node id -> Node id
         | Edge id -> Edge id
         | Cartesian { aggregation = Sum | Mean; _ }
         | Candlestick { aggregated = true; _ } ->
           Aggregate { source; data_revision; data_generation; selection })
  ;;
end

module Container = struct
  type t =
    | Card
    | Overlay
  [@@deriving bin_io, equal, sexp_of]
end

module Entry = struct
  type t =
    { target : Target.t option
    ; container : Container.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t = Entry.t list [@@deriving bin_io, equal, sexp_of]

let valid entries =
  List.length entries <= 128
  && List.for_all entries ~f:(fun entry ->
    Option.for_all entry.Entry.target ~f:Target.valid)
  && not
       (List.contains_dup
          (List.filter_map entries ~f:(fun e -> e.Entry.target))
          ~compare:Target.compare)
;;
