open Core

module Range = struct
  type t =
    { minimum : float
    ; maximum : float option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Float.is_finite t.minimum
    && Float.(t.minimum >= 0.)
    && Option.for_all t.maximum ~f:(fun maximum ->
      Float.is_finite maximum && Float.(maximum > t.minimum))
  ;;

  let contains t value =
    Float.is_finite value
    && Float.(value >= t.minimum)
    && Option.for_all t.maximum ~f:(fun maximum -> Float.(value < maximum))
  ;;
end

module Predicate = struct
  type t =
    { width : Range.t
    ; height : Range.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Range.valid t.width && Range.valid t.height

  let matches t ~width ~height =
    Range.contains t.width width && Range.contains t.height height
  ;;
end

module Rule = struct
  type t =
    { condition : Predicate.t
    ; branch : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

let valid_branch_id id =
  (not (String.is_empty id))
  && String.length id <= 128
  && Stdlib.String.is_valid_utf_8 id
  && not (String.contains id '\000')
;;

module Config = struct
  type t =
    { generation : int64
    ; branches : string list
    ; default : int64
    ; rules : Rule.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let count = List.length t.branches in
    let index value = Int64.(value >= 0L && value < of_int count) in
    Int64.(t.generation > 0L)
    && count > 0
    && count <= 16
    && List.for_all t.branches ~f:valid_branch_id
    && (not (List.contains_dup t.branches ~compare:String.compare))
    && index t.default
    && List.length t.rules <= 32
    && List.for_all t.rules ~f:(fun { Rule.condition; branch } ->
      Predicate.valid condition && index branch)
  ;;

  let select t ~width ~height =
    List.find_map t.rules ~f:(fun { Rule.condition; branch } ->
      Option.some_if (Predicate.matches condition ~width ~height) branch)
    |> Option.value ~default:t.default
  ;;
end

module Snapshot = struct
  type t =
    { generation : int64
    ; sequence : int64
    ; branch : int64
    ; width : float
    ; height : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.generation > 0L && t.sequence > 0L && t.branch >= 0L && t.branch < 16L)
    && Float.is_finite t.width
    && Float.is_finite t.height
    && Float.(t.width >= 0. && t.height >= 0.)
  ;;
end
