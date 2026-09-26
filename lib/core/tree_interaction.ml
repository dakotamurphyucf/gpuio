open Core
module Snapshot = Tree_loading.Snapshot
module State = Tree_state

module Target = struct
  type t =
    { lease : Tree_loading.Lease.t
    ; id : Tree.Id.t
    ; incarnation : int64
    }

  let capture snapshot id =
    match Tree.Expert.incarnation (Snapshot.tree snapshot) id with
    | None -> Or_error.error_string "tree interaction target is absent"
    | Some incarnation -> Ok { lease = Snapshot.lease snapshot; id; incarnation }
  ;;

  let id t = t.id

  let equal t other =
    Tree_loading.Lease.equal t.lease other.lease
    && Tree.Id.equal t.id other.id
    && Int64.equal t.incarnation other.incarnation
  ;;

  let is_current t snapshot =
    Tree_loading.Lease.equal t.lease (Snapshot.lease snapshot)
    && Option.exists
         (Tree.Expert.incarnation (Snapshot.tree snapshot) t.id)
         ~f:(Int64.equal t.incarnation)
  ;;
end

module Placement = struct
  type t =
    | Before
    | After
    | Inside
  [@@deriving equal, sexp_of]
end

let eligible state tree id =
  Option.is_some (State.visible_index state id)
  && Option.exists (Tree.find tree id) ~f:(fun node -> not (Tree.Node.is_disabled node))
;;

module Move = struct
  type t =
    { source : Target.t
    ; destination : Target.t
    ; placement : Placement.t
    }

  let source t = t.source
  let destination t = t.destination
  let placement t = t.placement

  let is_current t snapshot ~state =
    let tree = Snapshot.tree snapshot in
    let state = State.reconcile state tree in
    let source = Target.id t.source in
    let destination = Target.id t.destination in
    Target.is_current t.source snapshot
    && Target.is_current t.destination snapshot
    && (not (Tree.Id.equal source destination))
    && eligible state tree source
    && eligible state tree destination
    && (not
          (Option.exists (Tree.ancestors tree destination) ~f:(fun ancestors ->
             List.mem ancestors source ~equal:Tree.Id.equal)))
    &&
    match t.placement with
    | Before | After -> true
    | Inside ->
      Option.exists (Tree.find tree destination) ~f:(fun node ->
        match Tree.Node.children node with
        | Leaf -> false
        | Branch _ -> true)
  ;;
end

module Request = struct
  type t =
    | Navigate of Tree_loading.Lease.t * State.Selection.t option * State.Navigation.t
    | Select of Target.t * State.Selection.t
    | Focus of Target.t
    | Set_expanded of Target.t * bool
    | Activate of Target.t
    | Reveal of Target.t * bool
    | Move of Move.t
    | Select_active of Tree_loading.Lease.t * State.Selection.t
    | Activate_active of Tree_loading.Lease.t
    | Typeahead of Tree_loading.Lease.t * Tree_typeahead.Input.t
    | Set_selected of Target.t * bool

  let navigate snapshot ~selection direction =
    Navigate (Snapshot.lease snapshot, selection, direction)
  ;;

  let select target gesture = Select (target, gesture)
  let set_selected target selected = Set_selected (target, selected)
  let focus target = Focus target
  let set_expanded target expanded = Set_expanded (target, expanded)
  let activate target = Activate target
  let select_active snapshot selection = Select_active (Snapshot.lease snapshot, selection)
  let activate_active snapshot = Activate_active (Snapshot.lease snapshot)
  let typeahead snapshot input = Typeahead (Snapshot.lease snapshot, input)
  let reveal target ~focus = Reveal (target, focus)
  let move ~source ~destination placement = Move { source; destination; placement }
end

module Action = struct
  type t =
    | None
    | Activate of Tree.Id.t
    | Move of Move.t
end

module Outcome = struct
  type t =
    { state : State.t
    ; action : Action.t
    ; reveal : Target.t option
    ; focus : bool
    }

  let state t = t.state
  let action t = t.action
  let reveal t = t.reveal
  let focus t = t.focus
end

let apply state snapshot request =
  let tree = Snapshot.tree snapshot in
  let state = State.reconcile state tree in
  let result ?(action = Action.None) ?reveal ?(focus = false) state =
    Some { Outcome.state; action; reveal; focus }
  in
  let cursor state =
    let reveal =
      Option.bind (State.active state) ~f:(fun id ->
        Target.capture snapshot id |> Result.ok)
    in
    result ?reveal ~focus:(Option.is_some reveal) state
  in
  let targeted target f =
    if Target.is_current target snapshot && eligible state tree (Target.id target)
    then f (Target.id target)
    else None
  in
  match request with
  | Request.Navigate (lease, selection, direction) ->
    if Tree_loading.Lease.equal lease (Snapshot.lease snapshot)
    then cursor (State.navigate state tree ~selection direction)
    else None
  | Typeahead (lease, input) ->
    if Tree_loading.Lease.equal lease (Snapshot.lease snapshot)
    then (
      let state, found = State.typeahead state tree input in
      if Option.is_some found then cursor state else result state)
    else None
  | Select_active (lease, gesture) ->
    if Tree_loading.Lease.equal lease (Snapshot.lease snapshot)
    then
      Option.bind (State.active state) ~f:(fun id ->
        cursor (State.select state tree id gesture))
    else None
  | Activate_active lease ->
    if Tree_loading.Lease.equal lease (Snapshot.lease snapshot)
    then
      Option.bind (State.active state) ~f:(fun id -> result ~action:(Activate id) state)
    else None
  | Select (target, gesture) ->
    targeted target (fun id -> cursor (State.select state tree id gesture))
  | Set_selected (target, selected) ->
    targeted target (fun id -> result (State.set_selected state tree id selected))
  | Focus target -> targeted target (fun id -> cursor (State.focus state tree id))
  | Set_expanded (target, expanded) ->
    targeted target (fun id ->
      match Tree.Node.children (Tree.find tree id |> Option.value_exn) with
      | Leaf -> None
      | Branch _ ->
        let next =
          if Bool.equal expanded (State.is_expanded state id)
          then state
          else State.toggle_expanded state tree id
        in
        if Option.equal Tree.Id.equal (State.active state) (State.active next)
        then result next
        else cursor next)
  | Activate target -> targeted target (fun id -> result ~action:(Activate id) state)
  | Reveal (target, focus) ->
    if
      Target.is_current target snapshot
      && Option.exists
           (Tree.find tree (Target.id target))
           ~f:(fun node -> not (Tree.Node.is_disabled node))
    then result ~reveal:target ~focus (State.reveal state tree (Target.id target) ~focus)
    else None
  | Move proposal ->
    if Move.is_current proposal snapshot ~state
    then result ~action:(Move proposal) state
    else None
;;
