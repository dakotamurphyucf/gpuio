open Core

module Value = struct
  (* A fresh wrapper marks an explicit replacement, even when application data
     itself is physically shared. Structural reindexing preserves wrappers. *)
  type 'data t = { data : 'data }
end

type ('key, 'data, 'cmp) t =
  { order : 'key array
  ; keys : 'key list
  ; entries : ('key, int * 'data Value.t, 'cmp) Map.t
  }

let empty comparator = { order = [||]; keys = []; entries = Map.empty comparator }

let of_values comparator rows =
  let entries = List.mapi rows ~f:(fun index (key, data) -> key, (index, data)) in
  match Map.of_alist comparator entries with
  | `Duplicate_key _ -> Or_error.error_string "list collection keys must be unique"
  | `Ok entries ->
    let keys = List.map rows ~f:fst in
    Ok { order = Array.of_list keys; keys; entries }
;;

let of_alist comparator rows =
  of_values comparator (List.map rows ~f:(fun (key, data) -> key, { Value.data }))
;;

let length t = Array.length t.order
let keys t = t.keys
let is_empty t = length t = 0

let find t key =
  Map.find t.entries key |> Option.map ~f:(fun (_, value) -> value.Value.data)
;;

let index t key = Map.find t.entries key |> Option.map ~f:fst

let nth t index =
  if index < 0 || index >= length t
  then None
  else (
    let key = t.order.(index) in
    Some (key, (snd (Map.find_exn t.entries key)).Value.data))
;;

let range t ~first ~last =
  if first < 0 || last < first || last > length t
  then Or_error.error_string "list collection range is out of bounds"
  else
    Ok
      (List.init (last - first) ~f:(fun offset ->
         let key = t.order.(first + offset) in
         key, (snd (Map.find_exn t.entries key)).Value.data))
;;

let set t ~key ~data =
  match Map.find t.entries key with
  | None -> Or_error.error_string "cannot update an absent list collection key"
  | Some (index, _) ->
    Ok { t with entries = Map.set t.entries ~key ~data:(index, { Value.data }) }
;;

let to_alist t = range t ~first:0 ~last:(length t) |> Or_error.ok_exn

let fold_changed_keys t ~previous ~init ~f =
  Map.fold_symmetric_diff
    previous.entries
    t.entries
    ~data_equal:phys_equal
    ~init
    ~f:(fun acc (key, _) -> f acc key)
;;

let fold_changed_values t ~previous ~init ~f =
  Map.fold_symmetric_diff
    previous.entries
    t.entries
    ~data_equal:(fun (_, a) (_, b) -> phys_equal a b)
    ~init
    ~f:(fun acc (key, _) -> f acc key)
;;

let value_range t ~first ~last =
  List.init (last - first) ~f:(fun offset ->
    let key = t.order.(first + offset) in
    key, snd (Map.find_exn t.entries key))
;;

let splice t ~at ~remove rows =
  if at < 0 || at > length t || remove < 0 || remove > length t - at
  then Or_error.error_string "list collection splice is out of bounds"
  else (
    let before = value_range t ~first:0 ~last:at in
    let after = value_range t ~first:(at + remove) ~last:(length t) in
    let rows = List.map rows ~f:(fun (key, data) -> key, { Value.data }) in
    of_values (Map.comparator_s t.entries) (before @ rows @ after))
;;

let reorder t keys =
  if List.length keys <> length t
  then Or_error.error_string "list collection reorder must contain every key"
  else
    let open Or_error.Let_syntax in
    let%bind rows =
      List.map keys ~f:(fun key ->
        match Map.find t.entries key with
        | Some (_, value) -> Ok (key, value)
        | None -> Or_error.error_string "list collection reorder contains an absent key")
      |> Or_error.all
    in
    of_values (Map.comparator_s t.entries) rows
;;
