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

module Source_id = struct
  module T = struct
    type t = Type_equal.Id.Uid.t [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let create () =
    Type_equal.Id.create ~name:"table membership" sexp_of_unit |> Type_equal.Id.uid
  ;;
end

module Row_ref = struct
  module T = struct
    type t =
      { owner : Source_id.t
      ; id : Id.t
      ; lifetime : Source_id.t
      }
    [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let id t = t.id
end

module Identity = struct
  type t =
    { owner : Source_id.t
    ; keys : Id.t list
    ; lifetimes : (Id.t, Source_id.t, Id.comparator_witness) Map.t
    }

  let source_id t = t.owner

  let row_ref t id =
    Map.find t.lifetimes id
    |> Option.map ~f:(fun lifetime -> { Row_ref.owner = t.owner; id; lifetime })
  ;;

  let rows t = List.map t.keys ~f:(fun id -> row_ref t id |> Option.value_exn)
end

type 'data t =
  { identity : Identity.t
  ; revision : int64
  ; items : (Id.t, 'data, Id.comparator_witness) List_collection.t
  ; key_bytes : int
  }

let max_rows = Gpuio_protocol.List_wire.max_logical_rows
let max_key_bytes = 64 * 1024 * 1024
let length t = List_collection.length t.items
let is_empty t = List_collection.is_empty t.items
let revision t = t.revision
let key_bytes t = t.key_bytes
let same_source a b = Source_id.equal a.identity.owner b.identity.owner
let keys t = List_collection.keys t.items
let find t key = List_collection.find t.items key
let index t key = List_collection.index t.items key
let nth t index = List_collection.nth t.items index
let range t ~first ~last = List_collection.range t.items ~first ~last
let to_alist t = List_collection.to_alist t.items
let row_ref t key = Identity.row_ref t.identity key

let contains_ref t (reference : Row_ref.t) =
  Source_id.equal t.identity.owner reference.owner
  && Option.value_map
       (Map.find t.identity.lifetimes reference.id)
       ~default:false
       ~f:(Source_id.equal reference.lifetime)
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
      (List.map (List_collection.keys items) ~f:(fun key -> key, Source_id.create ()))
  in
  { identity =
      { Identity.owner = Source_id.create ()
      ; keys = List_collection.keys items
      ; lifetimes
      }
  ; revision = 0L
  ; items
  ; key_bytes
  }
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
           match Map.find t.identity.lifetimes key with
           | Some lifetime -> lifetime
           | None -> Source_id.create ()
         in
         key, lifetime))
  in
  { identity = { t.identity with keys = List_collection.keys items; lifetimes }
  ; revision
  ; items
  ; key_bytes
  }
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
  { t with
    revision
  ; items
  ; identity = { t.identity with keys = List_collection.keys items }
  }
;;

let fold_changed_values t ~previous ~init ~f =
  if not (same_source t previous)
  then Or_error.error_string "table change comparison requires the same source lineage"
  else Ok (List_collection.fold_changed_values t.items ~previous:previous.items ~init ~f)
;;

module Expert = struct
  module Identity = Identity

  let identity t = t.identity

  let row_key (row : Row_ref.t) =
    Source_id.sexp_of_t row.lifetime |> Sexp.to_string |> Key.of_string_exn
  ;;

  let items t = t.items
end
