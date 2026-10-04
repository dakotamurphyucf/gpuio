open Core

module Stamp = struct
  type t = Type_equal.Id.Uid.t [@@deriving compare, equal, sexp_of]

  let create () =
    Type_equal.Id.create ~name:"list membership" sexp_of_unit |> Type_equal.Id.uid
  ;;
end

module Source_id = struct
  module T = struct
    type t = Stamp.t [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let to_key t = Key.of_string_exn ("list-source:" ^ Sexp.to_string (sexp_of_t t))
end

module Item_ref = struct
  type 'key t =
    { source : Stamp.t
    ; key : 'key
    ; lifetime : Stamp.t
    }
  [@@deriving compare, equal, sexp_of]

  let key t = t.key
end

module Identity = struct
  type ('key, 'cmp) t =
    { source : Stamp.t
    ; order : 'key array
    ; keys : 'key list
    ; entries : ('key, int * Stamp.t, 'cmp) Map.t
    }

  let source_id t = t.source
  let same_source a b = Stamp.equal a.source b.source
  let length t = Array.length t.order
  let keys t = t.keys
  let index t key = Map.find t.entries key |> Option.map ~f:fst
  let nth t index = if index < 0 || index >= length t then None else Some t.order.(index)
  let comparator t = Map.comparator t.entries

  let item_ref t key =
    Map.find t.entries key
    |> Option.map ~f:(fun (_, lifetime) -> { Item_ref.source = t.source; key; lifetime })
  ;;

  let contains_ref t reference =
    Stamp.equal t.source reference.Item_ref.source
    && Option.exists (Map.find t.entries reference.key) ~f:(fun (_, lifetime) ->
      Stamp.equal lifetime reference.lifetime)
  ;;
end

module Value = struct
  (* A fresh wrapper marks explicit replacement while its membership stamp can
     survive. Reindexing preserves both wrapper and stamp. *)
  type 'data t =
    { data : 'data
    ; lifetime : Stamp.t
    }
end

type ('key, 'data, 'cmp) t =
  { identity : ('key, 'cmp) Identity.t
  ; entries : ('key, int * 'data Value.t, 'cmp) Map.t
  }

let of_values comparator ~source rows =
  let entries = List.mapi rows ~f:(fun index (key, data) -> key, (index, data)) in
  match Map.of_alist comparator entries with
  | `Duplicate_key _ -> Or_error.error_string "list collection keys must be unique"
  | `Ok entries ->
    let keys = List.map rows ~f:fst in
    let identity =
      { Identity.source
      ; order = Array.of_list keys
      ; keys
      ; entries = Map.map entries ~f:(fun (index, value) -> index, value.Value.lifetime)
      }
    in
    Ok { identity; entries }
;;

let empty comparator =
  of_values comparator ~source:(Stamp.create ()) [] |> Or_error.ok_exn
;;

let of_alist comparator rows =
  of_values
    comparator
    ~source:(Stamp.create ())
    (List.map rows ~f:(fun (key, data) -> key, { Value.data; lifetime = Stamp.create () }))
;;

let identity t = t.identity
let item_ref t = Identity.item_ref t.identity
let contains_ref t = Identity.contains_ref t.identity
let length t = Identity.length t.identity
let keys t = Identity.keys t.identity
let is_empty t = length t = 0

let find t key =
  Map.find t.entries key |> Option.map ~f:(fun (_, value) -> value.Value.data)
;;

let index t key = Map.find t.entries key |> Option.map ~f:fst

let nth t index =
  if index < 0 || index >= length t
  then None
  else (
    let key = t.identity.order.(index) in
    Some (key, (snd (Map.find_exn t.entries key)).Value.data))
;;

let range t ~first ~last =
  if first < 0 || last < first || last > length t
  then Or_error.error_string "list collection range is out of bounds"
  else
    Ok
      (List.init (last - first) ~f:(fun offset ->
         let key = t.identity.order.(first + offset) in
         key, (snd (Map.find_exn t.entries key)).Value.data))
;;

let set t ~key ~data =
  match Map.find t.entries key with
  | None -> Or_error.error_string "cannot update an absent list collection key"
  | Some (index, value) ->
    Ok { t with entries = Map.set t.entries ~key ~data:(index, { value with data }) }
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
    let key = t.identity.order.(first + offset) in
    key, snd (Map.find_exn t.entries key))
;;

let splice t ~at ~remove rows =
  if at < 0 || at > length t || remove < 0 || remove > length t - at
  then Or_error.error_string "list collection splice is out of bounds"
  else (
    let before = value_range t ~first:0 ~last:at in
    let after = value_range t ~first:(at + remove) ~last:(length t) in
    let rows =
      List.map rows ~f:(fun (key, data) ->
        let lifetime =
          match Map.find t.entries key with
          | Some (_, value) -> value.Value.lifetime
          | None -> Stamp.create ()
        in
        key, { Value.data; lifetime })
    in
    of_values
      (Map.comparator_s t.entries)
      ~source:t.identity.source
      (before @ rows @ after))
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
    of_values (Map.comparator_s t.entries) ~source:t.identity.source rows
;;

module Expert = struct
  let item_key reference =
    Stamp.sexp_of_t reference.Item_ref.lifetime |> Sexp.to_string |> Key.of_string_exn
  ;;
end
