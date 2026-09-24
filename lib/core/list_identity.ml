open Core
module W = Gpuio_protocol.List_wire

type t =
  { keys : Key.t list
  ; ids : int64 String.Map.t
  ; by_id : Key.t Int64.Map.t
  ; next : int64
  ; order : W.Order.t
  }

let id t key = Map.find t.ids (Key.to_string key)
let key t id = Map.find t.by_id id
let order t = t.order

let prepare previous source =
  let keys = Virtual_list.Order.keys source in
  match previous with
  | Some old when phys_equal old.keys keys || List.equal Key.equal old.keys keys -> Ok old
  | _ ->
    let open Or_error.Let_syntax in
    let revision = Option.value_map previous ~default:0L ~f:(fun t -> t.order.revision) in
    if Int64.equal revision Int64.max_value
    then Or_error.error_string "virtual list order revision exhausted"
    else (
      let next = Option.value_map previous ~default:1L ~f:(fun t -> t.next) in
      let%bind next, ids, by_id, runs =
        List.fold_result
          keys
          ~init:(next, String.Map.empty, Int64.Map.empty, [])
          ~f:(fun (next, ids, by_id, runs) key ->
            let old_id = Option.bind previous ~f:(fun t -> id t key) in
            let%map next, id =
              match old_id with
              | Some id -> Ok (next, id)
              | None ->
                if Int64.equal next Int64.max_value
                then Or_error.error_string "virtual list row identity exhausted"
                else Ok (Int64.succ next, next)
            in
            let runs =
              match runs with
              | run :: rest when Int64.equal Int64.(run.W.Id_run.first + run.count) id ->
                { run with count = Int64.succ run.count } :: rest
              | _ -> { W.Id_run.first = id; count = 1L } :: runs
            in
            ( next
            , Map.set ids ~key:(Key.to_string key) ~data:id
            , Map.set by_id ~key:id ~data:key
            , runs ))
      in
      let order : W.Order.t = { revision = Int64.succ revision; runs = List.rev runs } in
      let%map () = W.Order.validate order in
      { keys; ids; by_id; next; order })
;;
