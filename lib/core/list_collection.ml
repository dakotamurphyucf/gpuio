open Core

type ('key, 'data, 'cmp) t =
  { order : 'key array
  ; keys : 'key list
  ; entries : ('key, int * 'data, 'cmp) Map.t
  }

let empty comparator = { order = [||]; keys = []; entries = Map.empty comparator }

let of_alist comparator rows =
  let entries = List.mapi rows ~f:(fun index (key, data) -> key, (index, data)) in
  match Map.of_alist comparator entries with
  | `Duplicate_key _ -> Or_error.error_string "list collection keys must be unique"
  | `Ok entries ->
    let keys = List.map rows ~f:fst in
    Ok { order = Array.of_list keys; keys; entries }
;;

let length t = Array.length t.order
let keys t = t.keys
let is_empty t = length t = 0
let find t key = Map.find t.entries key |> Option.map ~f:snd
let index t key = Map.find t.entries key |> Option.map ~f:fst

let nth t index =
  if index < 0 || index >= length t
  then None
  else (
    let key = t.order.(index) in
    Some (key, snd (Map.find_exn t.entries key)))
;;

let range t ~first ~last =
  if first < 0 || last < first || last > length t
  then Or_error.error_string "list collection range is out of bounds"
  else
    Ok
      (List.init (last - first) ~f:(fun offset ->
         let key = t.order.(first + offset) in
         key, snd (Map.find_exn t.entries key)))
;;

let set t ~key ~data =
  match Map.find t.entries key with
  | None -> Or_error.error_string "cannot update an absent list collection key"
  | Some (index, _) -> Ok { t with entries = Map.set t.entries ~key ~data:(index, data) }
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

let splice t ~at ~remove rows =
  if at < 0 || at > length t || remove < 0 || remove > length t - at
  then Or_error.error_string "list collection splice is out of bounds"
  else (
    let before = range t ~first:0 ~last:at |> Or_error.ok_exn in
    let after = range t ~first:(at + remove) ~last:(length t) |> Or_error.ok_exn in
    of_alist (Map.comparator_s t.entries) (before @ rows @ after))
;;

let reorder t keys =
  if List.length keys <> length t
  then Or_error.error_string "list collection reorder must contain every key"
  else
    let open Or_error.Let_syntax in
    let%bind rows =
      List.map keys ~f:(fun key ->
        match find t key with
        | Some data -> Ok (key, data)
        | None -> Or_error.error_string "list collection reorder contains an absent key")
      |> Or_error.all
    in
    of_alist (Map.comparator_s t.entries) rows
;;
