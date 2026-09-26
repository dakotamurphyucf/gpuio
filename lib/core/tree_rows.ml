open Core
module View_key = Key
module Id = Tree.Id
module Snapshot = Tree_loading.Snapshot
module Collection = List_collection

module Key = struct
  module T = struct
    type t =
      { generation : int64
      ; serial : int64
      }
    [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let to_view_key t =
    View_key.of_string_exn
      ("tree:" ^ Int64.to_string t.generation ^ ":" ^ Int64.to_string t.serial)
  ;;
end

module Item = struct
  type 'data t =
    { id : Id.t
    ; node : 'data Tree.Node.t
    ; position : Tree.Position.t
    ; selected : bool
    ; expanded : bool option
    ; active : bool
    ; loading : Tree_loading.Status.t option
    }
end

module Boundary = struct
  type t =
    { parent : Id.t
    ; depth : int
    ; status : Tree_loading.Status.t
    }
end

module Row = struct
  type 'data t =
    | Item of 'data Item.t
    | Boundary of Boundary.t
end

module Identity = struct
  type t =
    { incarnation : int64
    ; key : Key.t
    }
end

type 'data t =
  { source : 'data Snapshot.t
  ; state : Tree_state.t
  ; collection : (Key.t, 'data Row.t, Key.comparator_witness) Collection.t
  ; items : (Id.t, Identity.t, Id.comparator_witness) Map.t
  ; boundaries : (Id.t, Identity.t, Id.comparator_witness) Map.t
  ; serial : int64
  }

let source t = t.source
let state t = t.state
let collection t = t.collection
let item_key t id = Map.find t.items id |> Option.map ~f:(fun i -> i.Identity.key)

let boundary_key t id =
  Map.find t.boundaries id |> Option.map ~f:(fun i -> i.Identity.key)
;;

let find t key = Collection.find t.collection key

let status_equal (a : Tree_loading.Status.t) (b : Tree_loading.Status.t) =
  match a, b with
  | Ready, Ready | Queued, Queued | Loading, Loading | End, End -> true
  | Failed a, Failed b ->
    phys_equal a b || String.equal (Error.to_string_hum a) (Error.to_string_hum b)
  | ( (Ready | Queued | Loading | End | Failed _)
    , (Ready | Queued | Loading | End | Failed _) ) -> false
;;

let row_equal a b =
  match a, b with
  | Row.Item a, Row.Item b ->
    Id.equal a.id b.id
    && phys_equal a.node b.node
    && Tree.Position.equal a.position b.position
    && Bool.equal a.selected b.selected
    && Option.equal Bool.equal a.expanded b.expanded
    && Bool.equal a.active b.active
    && Option.equal status_equal a.loading b.loading
  | Boundary a, Boundary b ->
    Id.equal a.parent b.parent && a.depth = b.depth && status_equal a.status b.status
  | Item _, Boundary _ | Boundary _, Item _ -> false
;;

let item source state id =
  let tree = Snapshot.tree source in
  let node = Tree.find tree id |> Option.value_exn in
  Row.Item
    { Item.id
    ; node
    ; position = Tree.position tree id |> Option.value_exn
    ; selected = Tree_state.is_selected state id
    ; expanded =
        (match Tree.Node.children node with
         | Leaf -> None
         | Branch _ -> Some (Tree_state.is_expanded state id))
    ; active = Option.exists (Tree_state.active state) ~f:(Id.equal id)
    ; loading = Snapshot.status source id
    }
;;

let boundary source id =
  let status = Snapshot.status source id |> Option.value_exn in
  let position = Tree.position (Snapshot.tree source) id |> Option.value_exn in
  Row.Boundary { Boundary.parent = id; depth = position.depth + 1; status }
;;

module Pending = struct
  type t =
    | Item of Id.t
    | Boundary of Id.t
end

let order source state =
  let tree = Snapshot.tree source in
  let rec walk pending reversed =
    match pending with
    | [] -> List.rev reversed
    | (Pending.Boundary _ as entry) :: rest -> walk rest (entry :: reversed)
    | (Item id as entry) :: rest ->
      let node = Tree.find tree id |> Option.value_exn in
      let rest =
        match Tree.Node.children node with
        | Branch { ids; next } when Tree_state.is_expanded state id ->
          let rest =
            match next with
            | End -> rest
            | More _ -> Pending.Boundary id :: rest
          in
          List.map ids ~f:(fun id -> Pending.Item id) @ rest
        | Leaf | Branch _ -> rest
      in
      walk rest (entry :: reversed)
  in
  walk (List.map (Tree.roots tree) ~f:(fun id -> Pending.Item id)) []
;;

let rebuild t source state =
  let tree = Snapshot.tree source in
  let entries = order source state in
  let count = List.length entries in
  if Int64.(t.serial > max_value - of_int count)
  then Or_error.error_string "tree row identity sequence exhausted"
  else (
    let serial, items, boundaries, rows =
      List.fold
        entries
        ~init:(t.serial, Map.empty (module Id), Map.empty (module Id), [])
        ~f:(fun (serial, items, boundaries, rows) entry ->
          let id, previous, row =
            match entry with
            | Pending.Item id -> id, t.items, item source state id
            | Boundary id -> id, t.boundaries, boundary source id
          in
          let incarnation = Tree.Expert.incarnation tree id |> Option.value_exn in
          let serial, identity =
            match Map.find previous id with
            | Some identity when Int64.equal identity.Identity.incarnation incarnation ->
              serial, identity
            | None | Some _ ->
              let serial = Int64.succ serial in
              ( serial
              , { Identity.incarnation
                ; key = { Key.generation = Snapshot.generation source; serial }
                } )
          in
          let items, boundaries =
            match entry with
            | Pending.Item _ -> Map.set items ~key:id ~data:identity, boundaries
            | Boundary _ -> items, Map.set boundaries ~key:id ~data:identity
          in
          serial, items, boundaries, (identity.key, row) :: rows)
    in
    let%map.Or_error collection = Collection.of_alist (module Key) (List.rev rows) in
    { source; state; collection; items; boundaries; serial })
;;

let create source ~state =
  let state = Tree_state.reconcile state (Snapshot.tree source) in
  let empty =
    { source
    ; state
    ; collection = Collection.empty (module Key)
    ; items = Map.empty (module Id)
    ; boundaries = Map.empty (module Id)
    ; serial = 0L
    }
  in
  rebuild empty source state
;;

let update_row collection key row =
  match Collection.find collection key with
  | Some previous when row_equal previous row -> collection
  | None -> assert false
  | Some _ -> Collection.set collection ~key ~data:row |> Or_error.ok_exn
;;

let update t source ~state =
  let tree = Snapshot.tree source in
  if not (Snapshot.same_generation source t.source)
  then Or_error.error_string "tree row projection requires a fresh generation"
  else if Int64.(Tree.revision tree < Tree.revision (Snapshot.tree t.source))
  then Or_error.error_string "tree row projection cannot move its source backwards"
  else (
    let state = Tree_state.reconcile state tree in
    if phys_equal source t.source && phys_equal state t.state
    then Ok t
    else if
      (not (phys_equal (Tree_state.visible state) (Tree_state.visible t.state)))
      || not (phys_equal (Tree.preorder tree) (Tree.preorder (Snapshot.tree t.source)))
    then rebuild t source state
    else (
      let changed =
        Snapshot.fold_changed_statuses
          source
          ~previous:t.source
          ~init:(Set.empty (module Id))
          ~f:Set.add
      in
      let changed =
        Tree_state.fold_changed_items state ~previous:t.state ~init:changed ~f:Set.add
      in
      let collection =
        Set.fold changed ~init:t.collection ~f:(fun collection id ->
          let collection =
            match item_key t id with
            | None -> collection
            | Some key -> update_row collection key (item source state id)
          in
          match boundary_key t id with
          | None -> collection
          | Some key -> update_row collection key (boundary source id))
      in
      Ok { t with source; state; collection }))
;;
