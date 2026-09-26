open Core

module Id = struct
  module T = struct
    type t = string [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let of_string text =
    if String.is_empty text || String.length text > 256
    then Or_error.error_string "table row id must contain 1..256 bytes"
    else if (not (Stdlib.String.is_valid_utf_8 text)) || String.contains text '\000'
    then Or_error.error_string "table row id must be UTF-8 without NUL"
    else Ok text
  ;;

  let to_string t = t
end

module Row_ref = struct
  type t =
    { owner : unit ref
    ; id : Id.t
    ; lifetime : unit ref
    }

  let id t = t.id

  let equal a b =
    phys_equal a.owner b.owner && Id.equal a.id b.id && phys_equal a.lifetime b.lifetime
  ;;
end

type 'data t =
  { owner : unit ref
  ; revision : int64
  ; items : (Id.t, 'data, Id.comparator_witness) List_collection.t
  ; lifetimes : (Id.t, unit ref, Id.comparator_witness) Map.t
  ; key_bytes : int
  }

let max_rows = Gpuio_protocol.List_wire.max_logical_rows
let max_key_bytes = 64 * 1024 * 1024
let length t = List_collection.length t.items
let is_empty t = List_collection.is_empty t.items
let revision t = t.revision
let key_bytes t = t.key_bytes
let same_source a b = phys_equal a.owner b.owner
let keys t = List_collection.keys t.items
let find t key = List_collection.find t.items key
let index t key = List_collection.index t.items key
let nth t index = List_collection.nth t.items index
let range t ~first ~last = List_collection.range t.items ~first ~last
let to_alist t = List_collection.to_alist t.items

let row_ref t key =
  Map.find t.lifetimes key
  |> Option.map ~f:(fun lifetime -> { Row_ref.owner = t.owner; id = key; lifetime })
;;

let contains_ref t (reference : Row_ref.t) =
  phys_equal t.owner reference.owner
  && Option.value_map
       (Map.find t.lifetimes reference.id)
       ~default:false
       ~f:(fun lifetime -> phys_equal lifetime reference.lifetime)
;;

let validate_count count =
  if count > max_rows
  then Or_error.errorf "table data exceeds %d rows" max_rows
  else Ok ()
;;

let count_key_bytes keys =
  List.fold_result keys ~init:0 ~f:(fun bytes key ->
    let bytes = bytes + String.length (Id.to_string key) in
    if bytes > max_key_bytes
    then Or_error.errorf "table data exceeds %d key bytes" max_key_bytes
    else Ok bytes)
;;

let admit rows =
  let%bind.Or_error () = validate_count (List.length rows) in
  let%bind.Or_error key_bytes = count_key_bytes (List.map rows ~f:fst) in
  let%map.Or_error items = List_collection.of_alist (module Id) rows in
  items, key_bytes
;;

let create rows =
  let%map.Or_error items, key_bytes = admit rows in
  let lifetimes =
    Map.of_alist_exn
      (module Id)
      (List.map (List_collection.keys items) ~f:(fun key -> key, ref ()))
  in
  { owner = ref (); revision = 0L; items; lifetimes; key_bytes }
;;

let next_revision t =
  if Int64.equal t.revision Int64.max_value
  then Or_error.error_string "table data revision exhausted"
  else Ok (Int64.succ t.revision)
;;

let set t ~key ~data =
  let%bind.Or_error revision = next_revision t in
  let%map.Or_error items = List_collection.set t.items ~key ~data in
  { t with revision; items }
;;

let replace_items t ~revision ~items ~key_bytes =
  let lifetimes =
    Map.of_alist_exn
      (module Id)
      (List.map (List_collection.keys items) ~f:(fun key ->
         let lifetime =
           match Map.find t.lifetimes key with
           | Some lifetime -> lifetime
           | None -> ref ()
         in
         key, lifetime))
  in
  { t with revision; items; lifetimes; key_bytes }
;;

let replace t rows =
  let%bind.Or_error revision = next_revision t in
  let%map.Or_error items, key_bytes = admit rows in
  replace_items t ~revision ~items ~key_bytes
;;

let splice t ~at ~remove rows =
  if at < 0 || at > length t || remove < 0 || remove > length t - at
  then Or_error.error_string "table data splice is out of bounds"
  else (
    let%bind.Or_error revision = next_revision t in
    let%bind.Or_error () = validate_count (length t - remove + List.length rows) in
    let%bind.Or_error items = List_collection.splice t.items ~at ~remove rows in
    let%map.Or_error key_bytes = count_key_bytes (List_collection.keys items) in
    replace_items t ~revision ~items ~key_bytes)
;;

let reorder t keys =
  let%bind.Or_error revision = next_revision t in
  let%map.Or_error items = List_collection.reorder t.items keys in
  { t with revision; items }
;;

let fold_changed_values t ~previous ~init ~f =
  if not (same_source t previous)
  then Or_error.error_string "table change comparison requires the same source lineage"
  else Ok (List_collection.fold_changed_values t.items ~previous:previous.items ~init ~f)
;;

module Expert = struct
  let items t = t.items
end
