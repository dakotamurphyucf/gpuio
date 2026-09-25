open Core
module W = Gpuio_protocol.Container_query_wire

module Branch_id = struct
  type t = string [@@deriving compare, equal, sexp_of]

  let of_string value =
    if W.valid_branch_id value
    then Ok value
    else
      Or_error.error_string
        "container branch ID must contain 1..128 UTF-8 bytes without NUL"
  ;;

  let to_string t = t
end

module Range = struct
  type t = W.Range.t [@@deriving equal, sexp_of]

  let create ?(minimum = 0.) ?maximum () =
    let t = { W.Range.minimum; maximum } in
    if W.Range.valid t
    then Ok t
    else
      Or_error.error_string "container range needs finite nonnegative minimum < maximum"
  ;;

  let all = { W.Range.minimum = 0.; maximum = None }
end

module Predicate = struct
  type t = W.Predicate.t [@@deriving equal, sexp_of]

  let create ?(width = Range.all) ?(height = Range.all) () = { W.Predicate.width; height }
end

module Rule = struct
  type t =
    { condition : Predicate.t
    ; branch : Branch_id.t
    }
  [@@deriving equal, sexp_of]

  let create ~condition ~branch = { condition; branch }
end

module Config = struct
  type t =
    { default : Branch_id.t
    ; rules : Rule.t list
    ; branches : Branch_id.t list
    }
  [@@deriving equal, sexp_of]

  let create ~default rules =
    if List.length rules > 32
    then Or_error.error_string "container queries allow at most 32 rules"
    else (
      let branches =
        default :: List.map rules ~f:(fun rule -> rule.Rule.branch)
        |> List.fold
             ~init:(Set.empty (module String), [])
             ~f:(fun (seen, result) branch ->
               if Set.mem seen branch
               then seen, result
               else Set.add seen branch, branch :: result)
        |> snd
        |> List.rev
      in
      if List.length branches > 16
      then Or_error.error_string "container queries allow at most 16 branches"
      else Ok { default; rules; branches })
  ;;

  let branches t = t.branches

  let select t ~width ~height =
    if
      not
        (Float.is_finite width
         && Float.is_finite height
         && Float.(width >= 0. && height >= 0.))
    then Or_error.error_string "assigned container size must be finite and nonnegative"
    else
      Ok
        (List.find_map t.rules ~f:(fun { Rule.condition; branch } ->
           Option.some_if (W.Predicate.matches condition ~width ~height) branch)
         |> Option.value ~default:t.default)
  ;;
end

module Expert = struct
  let to_wire (t : Config.t) ~generation =
    let index branch =
      List.findi_exn t.branches ~f:(fun _ candidate -> Branch_id.equal candidate branch)
      |> fst
      |> Int64.of_int
    in
    let config =
      { W.Config.generation
      ; branches = List.map t.branches ~f:Branch_id.to_string
      ; default = index t.default
      ; rules =
          List.map t.rules ~f:(fun { Rule.condition; branch } ->
            { W.Rule.condition; branch = index branch })
      }
    in
    if W.Config.valid config
    then Ok config
    else Or_error.error_string "invalid container query admission generation"
  ;;
end
