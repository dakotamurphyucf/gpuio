open Core
module View_key = Key
module C = List_collection
module Catalog = List_selection.Catalog

module Key = struct
  module T = struct
    type t = string [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let to_view_key = View_key.of_string_exn
  let of_target target = C.Expert.item_key target |> View_key.to_string
end

module Kind = struct
  type t =
    | Option of
        { index : int
        ; count : int
        }
    | Decoration
  [@@deriving equal, sexp_of]
end

module Layout = struct
  type ('key, 'cmp) t =
    { catalog : ('key, 'cmp) Catalog.t
    ; kinds : ('key, Kind.t, 'cmp) Map.t
    ; option_count : int
    }

  let catalog t = t.catalog
  let kind t key = Map.find t.kinds key
  let option_count t = t.option_count

  let create catalog ?(decorations = []) () =
    let identity = Catalog.identity catalog in
    if List.length decorations > C.Identity.length identity
    then Or_error.error_string "list decorations exceed loaded membership"
    else (
      let comparator = C.Identity.comparator identity in
      let set = Set.Using_comparator.of_list ~comparator decorations in
      if
        Set.length set <> List.length decorations
        || not
             (List.for_all decorations ~f:(fun key ->
                Option.is_some (C.Identity.index identity key)
                && not (Catalog.is_enabled catalog key)))
      then Or_error.error_string "list decorations must be unique loaded ineligible keys"
      else (
        let visible = Catalog.visible catalog in
        let option_count = List.count visible ~f:(fun key -> not (Set.mem set key)) in
        let _, kinds =
          List.fold
            visible
            ~init:(0, Map.Using_comparator.empty ~comparator)
            ~f:(fun (index, kinds) key ->
              let index, kind =
                if Set.mem set key
                then index, Kind.Decoration
                else index + 1, Kind.Option { index; count = option_count }
              in
              index, Map.set kinds ~key ~data:kind)
        in
        Ok { catalog; kinds; option_count }))
  ;;
end

module Item = struct
  type ('key, 'data) t =
    { target : 'key C.Item_ref.t
    ; data : 'data
    ; kind : Kind.t
    ; disabled : bool
    }

  let target t = t.target
  let key t = C.Item_ref.key t.target
  let data t = t.data
  let kind t = t.kind
  let is_disabled t = t.disabled

  let accessibility t ~label ~selected =
    match t.kind with
    | Kind.Decoration -> Ok None
    | Option { index; count } ->
      let open Or_error.Let_syntax in
      let%bind item =
        Accessibility.Option_item.create ~index ~count ~selected ~disabled:t.disabled ()
      in
      let%map accessibility = Accessibility.create ~role:(Option_item item) ~label () in
      Some accessibility
  ;;
end

type ('key, 'data, 'cmp) t =
  { source : ('key, 'data, 'cmp) C.t
  ; layout : ('key, 'cmp) Layout.t
  ; collection : (Key.t, ('key, 'data) Item.t, Key.comparator_witness) C.t
  }

let source t = t.source
let layout t = t.layout
let collection t = t.collection
let find t key = C.find t.collection key

let item_key t target =
  if not (C.contains_ref t.source target)
  then None
  else (
    let key = Key.of_target target in
    if Option.is_some (find t key) then Some key else None)
;;

let valid source layout =
  phys_equal (C.identity source) (Catalog.identity (Layout.catalog layout))
;;

let item source layout key =
  { Item.target = C.item_ref source key |> Option.value_exn
  ; data = C.find source key |> Option.value_exn
  ; kind = Layout.kind layout key |> Option.value_exn
  ; disabled = not (Catalog.is_enabled (Layout.catalog layout) key)
  }
;;

let same_item a b =
  Key.equal (Key.of_target a.Item.target) (Key.of_target b.Item.target)
  && phys_equal a.data b.data
  && Kind.equal a.kind b.kind
  && Bool.equal a.disabled b.disabled
;;

let rebuild previous source layout =
  let rows =
    List.map
      (Catalog.visible (Layout.catalog layout))
      ~f:(fun app_key ->
        let row = item source layout app_key in
        let key = Key.of_target row.target in
        let row =
          match Option.bind previous ~f:(fun old -> find old key) with
          | Some old when same_item old row -> old
          | Some _ | None -> row
        in
        key, row)
  in
  let%map.Or_error collection =
    match previous with
    | Some old when C.Identity.same_source (C.identity old.source) (C.identity source) ->
      C.splice old.collection ~at:0 ~remove:(C.length old.collection) rows
    | Some _ | None -> C.of_alist (module Key) rows
  in
  { source; layout; collection }
;;

let create source ~layout =
  if not (valid source layout)
  then Or_error.error_string "list layout refers to an obsolete source identity"
  else rebuild None source layout
;;

let update t source ~layout =
  if not (valid source layout)
  then Or_error.error_string "list layout refers to an obsolete source identity"
  else if not (phys_equal t.layout layout)
  then rebuild (Some t) source layout
  else if phys_equal t.source source
  then Ok t
  else (
    let collection =
      C.fold_changed_values
        source
        ~previous:t.source
        ~init:t.collection
        ~f:(fun collection app_key ->
          match C.item_ref source app_key with
          | None -> collection
          | Some target ->
            let key = Key.of_target target in
            if Option.is_none (C.find collection key)
            then collection
            else
              C.set collection ~key ~data:(item source layout app_key) |> Or_error.ok_exn)
    in
    Ok { t with source; collection })
;;
