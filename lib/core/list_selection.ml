open Core
module Identity = List_collection.Identity
module Item_ref = List_collection.Item_ref

module Mode = struct
  type t =
    | Single
    | Multiple
  [@@deriving equal, sexp_of]
end

module Boundary = struct
  type t =
    | Stop
    | Wrap
  [@@deriving equal, sexp_of]
end

module Navigation = struct
  type t =
    | Previous
    | Next
    | First
    | Last
  [@@deriving equal, sexp_of]
end

module Gesture = struct
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving equal, sexp_of]
end

module Confirmation = struct
  type t =
    | Primary
    | Secondary
  [@@deriving equal, sexp_of]
end

module Catalog = struct
  type ('key, 'cmp) t =
    { identity : ('key, 'cmp) Identity.t
    ; visible : 'key list
    ; positions : ('key, int, 'cmp) Map.t
    ; enabled : 'key Int.Map.t
    }

  let identity t = t.identity
  let visible t = t.visible
  let visible_index t key = Map.find t.positions key
  let is_enabled t key = Option.exists (visible_index t key) ~f:(Map.mem t.enabled)

  let empty_map t =
    Map.Using_comparator.empty ~comparator:(Identity.comparator t.identity)
  ;;

  let reference t key = Identity.item_ref t.identity key |> Option.value_exn

  let eligible t reference =
    Identity.contains_ref t.identity reference && is_enabled t (Item_ref.key reference)
  ;;

  let create identity ?visible ?(disabled = []) () =
    let max = Gpuio_protocol.List_wire.max_logical_rows in
    let visible = Option.value visible ~default:(Identity.keys identity) in
    if
      Identity.length identity > max
      || List.length visible > max
      || List.length disabled > max
    then Or_error.error_string "list selection catalog exceeds the logical-item limit"
    else (
      let comparator = Identity.comparator identity in
      let disabled_set = Set.Using_comparator.of_list ~comparator disabled in
      if
        Set.length disabled_set <> List.length disabled
        || not
             (List.for_all disabled ~f:(fun key ->
                Option.is_some (Identity.index identity key)))
      then Or_error.error_string "disabled list keys must be unique loaded members"
      else
        let open Or_error.Let_syntax in
        let%map positions, enabled =
          List.foldi
            visible
            ~init:(Ok (Map.Using_comparator.empty ~comparator, Int.Map.empty))
            ~f:(fun index result key ->
              let%bind positions, enabled = result in
              if Map.mem positions key || Option.is_none (Identity.index identity key)
              then Or_error.error_string "visible list keys must be unique loaded members"
              else
                Ok
                  ( Map.set positions ~key ~data:index
                  , if Set.mem disabled_set key
                    then enabled
                    else Map.set enabled ~key:index ~data:key ))
        in
        { identity; visible; positions; enabled })
  ;;
end

type ('key, 'cmp) t =
  { catalog : ('key, 'cmp) Catalog.t
  ; mode : Mode.t
  ; cursor : 'key Item_ref.t option
  ; selected : ('key, 'key Item_ref.t, 'cmp) Map.t
  ; anchor : 'key Item_ref.t option
  ; context : 'key Item_ref.t option
  }

let mode t = t.mode
let cursor t = t.cursor
let selected t = Map.data t.selected
let is_selected t key = Map.mem t.selected key
let anchor t = t.anchor
let context t = t.context

let empty catalog mode =
  { catalog
  ; mode
  ; cursor = None
  ; selected = Catalog.empty_map catalog
  ; anchor = None
  ; context = None
  }
;;

let reconcile t catalog =
  if phys_equal t.catalog catalog
  then t
  else if not (Identity.same_source t.catalog.identity catalog.Catalog.identity)
  then empty catalog t.mode
  else (
    let selected =
      if phys_equal t.catalog.identity catalog.identity
      then t.selected
      else Map.filter t.selected ~f:(Identity.contains_ref catalog.identity)
    in
    let retain reference = Option.filter reference ~f:(Catalog.eligible catalog) in
    let cursor =
      match t.cursor with
      | None -> None
      | Some reference when Catalog.eligible catalog reference -> Some reference
      | Some reference ->
        let index =
          Catalog.visible_index t.catalog (Item_ref.key reference)
          |> Option.value ~default:0
        in
        let next =
          match Map.closest_key catalog.enabled `Greater_or_equal_to index with
          | Some item -> Some item
          | None -> Map.closest_key catalog.enabled `Less_or_equal_to index
        in
        Option.map next ~f:(fun (_, key) -> Catalog.reference catalog key)
    in
    { t with
      catalog
    ; selected
    ; cursor
    ; anchor = retain t.anchor
    ; context = retain t.context
    })
;;

let with_selected t catalog keys =
  let t = reconcile t catalog in
  if List.length keys > Identity.length catalog.identity
  then Or_error.error_string "selected list keys exceed loaded membership"
  else if Mode.equal t.mode Single && List.length keys > 1
  then Or_error.error_string "single list selection accepts at most one item"
  else
    let open Or_error.Let_syntax in
    let%map selected =
      List.fold_result keys ~init:(Catalog.empty_map catalog) ~f:(fun selected key ->
        if Map.mem selected key
        then Or_error.error_string "selected list keys must be unique"
        else (
          match Identity.item_ref catalog.identity key with
          | None -> Or_error.error_string "selected list key is not loaded"
          | Some reference -> Ok (Map.set selected ~key ~data:reference)))
    in
    { t with selected; anchor = None }
;;

let create catalog ?(mode = Mode.Single) ?(selected = []) () =
  with_selected (empty catalog mode) catalog selected
;;

let with_mode t catalog mode =
  let t = reconcile t catalog in
  if Mode.equal t.mode mode
  then t
  else (
    let selected =
      match mode with
      | Multiple -> t.selected
      | Single ->
        let chosen =
          match
            Option.bind t.cursor ~f:(fun reference ->
              Map.find t.selected (Item_ref.key reference))
          with
          | Some reference -> Some reference
          | None ->
            Map.fold t.selected ~init:None ~f:(fun ~key ~data candidate ->
              let index = Identity.index catalog.identity key |> Option.value_exn in
              match candidate with
              | Some (previous, _) when previous <= index -> candidate
              | None | Some _ -> Some (index, data))
            |> Option.map ~f:snd
        in
        Option.value_map chosen ~default:(Catalog.empty_map catalog) ~f:(fun reference ->
          Map.set
            (Catalog.empty_map catalog)
            ~key:(Item_ref.key reference)
            ~data:reference)
    in
    { t with mode; selected; anchor = None })
;;

let focus t catalog reference =
  let t = reconcile t catalog in
  if Catalog.eligible catalog reference then { t with cursor = Some reference } else t
;;

let select t catalog reference gesture =
  let t = reconcile t catalog in
  if not (Catalog.eligible catalog reference)
  then t
  else (
    let gesture =
      match t.mode with
      | Single -> Gesture.Replace
      | Multiple -> gesture
    in
    let key = Item_ref.key reference in
    let selected, anchor =
      match gesture with
      | Replace ->
        Map.set (Catalog.empty_map catalog) ~key ~data:reference, Some reference
      | Toggle ->
        ( (if Map.mem t.selected key
           then Map.remove t.selected key
           else Map.set t.selected ~key ~data:reference)
        , Some reference )
      | Range { extend } ->
        let anchor =
          match t.anchor with
          | Some anchor when Catalog.eligible catalog anchor -> anchor
          | None | Some _ -> Option.value t.cursor ~default:reference
        in
        let start =
          Catalog.visible_index catalog (Item_ref.key anchor) |> Option.value_exn
        in
        let stop = Catalog.visible_index catalog key |> Option.value_exn in
        let selected =
          Map.fold_range_inclusive
            catalog.enabled
            ~min:(Int.min start stop)
            ~max:(Int.max start stop)
            ~init:(if extend then t.selected else Catalog.empty_map catalog)
            ~f:(fun ~key:_ ~data:key selected ->
              Map.set selected ~key ~data:(Catalog.reference catalog key))
        in
        selected, Some anchor
    in
    { t with selected; anchor; cursor = Some reference })
;;

let set_selected t catalog reference selected =
  let t = reconcile t catalog in
  if not (Catalog.eligible catalog reference)
  then t
  else (
    let key = Item_ref.key reference in
    let selected =
      if not selected
      then Map.remove t.selected key
      else
        Map.set
          (match t.mode with
           | Single -> Catalog.empty_map catalog
           | Multiple -> t.selected)
          ~key
          ~data:reference
    in
    { t with selected })
;;

let navigate t catalog ?(boundary = Boundary.Stop) ?selection navigation =
  let t = reconcile t catalog in
  let first () = Map.min_elt catalog.Catalog.enabled in
  let last () = Map.max_elt catalog.enabled in
  let next direction fallback =
    match
      Option.bind t.cursor ~f:(fun reference ->
        Catalog.visible_index catalog (Item_ref.key reference))
    with
    | None -> fallback ()
    | Some index ->
      (match Map.closest_key catalog.enabled direction index with
       | Some _ as found -> found
       | None ->
         (match boundary with
          | Stop -> None
          | Wrap -> fallback ()))
  in
  let target =
    match navigation with
    | Navigation.First -> first ()
    | Last -> last ()
    | Next -> next `Greater_than first
    | Previous -> next `Less_than last
  in
  match target with
  | None -> t
  | Some (_, key) ->
    let reference = Catalog.reference catalog key in
    (match selection with
     | None -> { t with cursor = Some reference }
     | Some gesture -> select t catalog reference gesture)
;;

let with_context t catalog target =
  let t = reconcile t catalog in
  match target with
  | None -> { t with context = None }
  | Some reference ->
    if Catalog.eligible catalog reference then { t with context = target } else t
;;

let confirm t catalog kind =
  let t = reconcile t catalog in
  Option.map t.cursor ~f:(fun reference -> reference, kind)
;;

let cancel t catalog =
  let t = reconcile t catalog in
  { t with cursor = None; anchor = None; context = None }
;;
