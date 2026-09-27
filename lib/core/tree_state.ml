open Core
module Id = Tree.Id

module Mode = struct
  type t =
    | Single
    | Multiple
  [@@deriving equal, sexp_of]
end

module Selection = struct
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving equal, sexp_of]
end

module Navigation = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Parent
    | Child
  [@@deriving equal, sexp_of]
end

type versions = (Id.t, int64, Id.comparator_witness) Map.t

module Row = struct
  type t =
    { id : Id.t
    ; parent : Id.t option
    ; incarnation : int64
    ; disabled : bool
    }
end

type t =
  { mode : Mode.t
  ; selected : versions
  ; expanded : versions
  ; active : Id.t option
  ; anchor : (Id.t * int64) option
  ; typeahead : Tree_typeahead.t
  ; source_order : Id.t list
  ; visible : Id.t list
  ; rows : Row.t array
  ; indices : (Id.t, int, Id.comparator_witness) Map.t
  }

let empty = Map.empty (module Id)
let mode t = t.mode
let selected t = Map.keys t.selected
let expanded t = Map.keys t.expanded
let is_selected t id = Map.mem t.selected id
let is_expanded t id = Map.mem t.expanded id
let active t = t.active
let anchor t = Option.map t.anchor ~f:fst
let visible t = t.visible
let visible_index t id = Map.find t.indices id
let row t id = Option.map (visible_index t id) ~f:(Array.get t.rows)
let eligible t id = Option.exists (row t id) ~f:(fun row -> not row.Row.disabled)

let fold_changed_items t ~previous ~init ~f =
  let diff before after changed =
    Map.fold_symmetric_diff
      before
      after
      ~data_equal:Int64.equal
      ~init:changed
      ~f:(fun changed (id, _) -> Set.add changed id)
  in
  let changed =
    Set.empty (module Id)
    |> diff previous.selected t.selected
    |> diff previous.expanded t.expanded
  in
  let changed =
    if Option.equal Id.equal previous.active t.active
    then changed
    else
      List.fold
        (Option.to_list previous.active @ Option.to_list t.active)
        ~init:changed
        ~f:Set.add
  in
  Set.fold changed ~init ~f
;;

let branch node =
  match Tree.Node.children node with
  | Leaf -> false
  | Branch _ -> true
;;

let versions tree ~branches ids =
  List.fold ids ~init:(Ok empty) ~f:(fun result id ->
    let%bind.Or_error result = result in
    match Tree.find tree id with
    | None -> Or_error.error_string "tree preference references an absent node"
    | Some node ->
      if Map.mem result id
      then Or_error.error_string "tree preference IDs must be unique"
      else if branches && not (branch node)
      then Or_error.error_string "a tree leaf cannot be expanded"
      else
        Ok
          (Map.set
             result
             ~key:id
             ~data:(Tree.Expert.incarnation tree id |> Option.value_exn)))
;;

let project tree expanded =
  let rec walk pending reversed =
    match pending with
    | [] -> List.rev reversed
    | id :: rest ->
      let node = Tree.find tree id |> Option.value_exn in
      let parent = (Tree.position tree id |> Option.value_exn).Tree.Position.parent in
      let row =
        { Row.id
        ; parent
        ; incarnation = Tree.Expert.incarnation tree id |> Option.value_exn
        ; disabled = Tree.Node.is_disabled node
        }
      in
      let children =
        match Tree.Node.children node with
        | Branch { ids; _ } when Map.mem expanded id -> ids
        | Leaf | Branch _ -> []
      in
      walk (children @ rest) (row :: reversed)
  in
  let rows = walk (Tree.roots tree) [] in
  let visible = List.map rows ~f:(fun row -> row.Row.id) in
  let indices =
    Map.of_alist_exn (module Id) (List.mapi visible ~f:(fun index id -> id, index))
  in
  Array.of_list rows, visible, indices
;;

let create tree ?(mode = Mode.Single) ?(selected = []) ?(expanded = []) () =
  let open Or_error.Let_syntax in
  let%bind selected = versions tree ~branches:false selected in
  let%bind expanded = versions tree ~branches:true expanded in
  if Mode.equal mode Single && Map.length selected > 1
  then Or_error.error_string "single tree selection has more than one node"
  else (
    let rows, visible, indices = project tree expanded in
    Ok
      { mode
      ; selected
      ; expanded
      ; active = None
      ; anchor = None
      ; typeahead = Tree_typeahead.empty
      ; source_order = Tree.preorder tree
      ; rows
      ; visible
      ; indices
      })
;;

let current tree id incarnation =
  Option.exists (Tree.Expert.incarnation tree id) ~f:(Int64.equal incarnation)
;;

let old_ancestors t id =
  let rec collect id reversed =
    match row t id with
    | None | Some { Row.parent = None; _ } -> List.rev reversed
    | Some { parent = Some parent; _ } -> collect parent (parent :: reversed)
  in
  collect id []
;;

let fallback_neighbor t index =
  let count = Array.length t.rows in
  let rec forward index =
    if index >= count
    then None
    else if t.rows.(index).disabled
    then forward (index + 1)
    else Some t.rows.(index).id
  in
  let rec backward index =
    if index < 0
    then None
    else if t.rows.(index).disabled
    then backward (index - 1)
    else Some t.rows.(index).id
  in
  match forward index with
  | Some _ as found -> found
  | None -> backward (Int.min (index - 1) (count - 1))
;;

let repair_active previous next tree =
  match previous.active with
  | None -> None
  | Some id ->
    let same =
      Option.exists (row previous id) ~f:(fun row -> current tree id row.incarnation)
    in
    if same && eligible next id
    then Some id
    else (
      let ancestors =
        if same
        then Tree.ancestors tree id |> Option.value_exn |> List.rev
        else old_ancestors previous id
      in
      match List.find ancestors ~f:(eligible next) with
      | Some _ as ancestor -> ancestor
      | None ->
        fallback_neighbor next (Option.value (visible_index previous id) ~default:0))
;;

let rebuild t tree =
  let rows, visible, indices = project tree t.expanded in
  let next = { t with rows; visible; indices; source_order = Tree.preorder tree } in
  { next with active = repair_active t next tree }
;;

let reconcile t tree =
  if phys_equal t.source_order (Tree.preorder tree)
  then t
  else (
    let selected = Map.filteri t.selected ~f:(fun ~key ~data -> current tree key data) in
    let expanded =
      Map.filteri t.expanded ~f:(fun ~key ~data ->
        current tree key data && Option.exists (Tree.find tree key) ~f:branch)
    in
    let anchor =
      Option.filter t.anchor ~f:(fun (id, incarnation) -> current tree id incarnation)
    in
    rebuild { t with selected; expanded; anchor } tree)
;;

let with_selected t tree ids =
  let t = reconcile t tree in
  let%bind.Or_error selected = versions tree ~branches:false ids in
  if Mode.equal t.mode Single && Map.length selected > 1
  then Or_error.error_string "single tree selection has more than one node"
  else Ok { t with selected; anchor = None }
;;

let with_expanded t tree ids =
  let t = reconcile t tree in
  let%map.Or_error expanded = versions tree ~branches:true ids in
  if Map.equal Int64.equal expanded t.expanded
  then t
  else rebuild { t with expanded } tree
;;

let with_mode t tree mode =
  let t = reconcile t tree in
  if Mode.equal mode t.mode
  then t
  else (
    let selected =
      match mode with
      | Multiple -> t.selected
      | Single ->
        let target =
          match Option.filter t.active ~f:(Map.mem t.selected) with
          | Some _ as id -> id
          | None -> List.find (Tree.preorder tree) ~f:(Map.mem t.selected)
        in
        Option.value_map target ~default:empty ~f:(fun id ->
          Map.singleton (module Id) id (Map.find_exn t.selected id))
    in
    { t with mode; selected; anchor = None })
;;

let set_selected t tree id selected =
  let t = reconcile t tree in
  if not (eligible t id)
  then t
  else (
    let selected =
      if selected
      then (
        let incarnation = (row t id |> Option.value_exn).incarnation in
        match t.mode with
        | Single -> Map.singleton (module Id) id incarnation
        | Multiple -> Map.set t.selected ~key:id ~data:incarnation)
      else Map.remove t.selected id
    in
    { t with selected })
;;

let focus t tree id =
  let t = reconcile t tree in
  if eligible t id then { t with active = Some id } else t
;;

let toggle_expanded t tree id =
  let t = reconcile t tree in
  if (not (eligible t id)) || not (Option.exists (Tree.find tree id) ~f:branch)
  then t
  else (
    let expanded =
      if Map.mem t.expanded id
      then Map.remove t.expanded id
      else
        Map.set
          t.expanded
          ~key:id
          ~data:(Tree.Expert.incarnation tree id |> Option.value_exn)
    in
    rebuild { t with expanded } tree)
;;

let range t anchor target ~extend =
  let a = Map.find_exn t.indices anchor in
  let b = Map.find_exn t.indices target in
  let rec collect index selected =
    if index > Int.max a b
    then selected
    else (
      let row = t.rows.(index) in
      let selected =
        if row.disabled
        then selected
        else Map.set selected ~key:row.id ~data:row.incarnation
      in
      collect (index + 1) selected)
  in
  collect (Int.min a b) (if extend then t.selected else empty)
;;

let select t tree id gesture =
  let t = reconcile t tree in
  if not (eligible t id)
  then t
  else (
    let incarnation = (row t id |> Option.value_exn).incarnation in
    let gesture = if Mode.equal t.mode Single then Selection.Replace else gesture in
    let selected, anchor =
      match gesture with
      | Replace -> Map.singleton (module Id) id incarnation, Some (id, incarnation)
      | Toggle ->
        ( (if Map.mem t.selected id
           then Map.remove t.selected id
           else Map.set t.selected ~key:id ~data:incarnation)
        , Some (id, incarnation) )
      | Range { extend } ->
        let anchor =
          match Option.filter (anchor t) ~f:(eligible t) with
          | Some anchor -> anchor
          | None -> Option.value (Option.filter t.active ~f:(eligible t)) ~default:id
        in
        ( range t anchor id ~extend
        , Some (anchor, (row t anchor |> Option.value_exn).incarnation) )
    in
    { t with selected; anchor; active = Some id })
;;

let navigate t tree ~selection direction =
  let t = reconcile t tree in
  let rec seek index step =
    if index < 0 || index >= Array.length t.rows
    then None
    else if t.rows.(index).disabled
    then seek (index + step) step
    else Some t.rows.(index).id
  in
  let destination id =
    match selection with
    | None -> focus t tree id
    | Some gesture -> select t tree id gesture
  in
  let move = Option.value_map ~default:t ~f:destination in
  let first () = seek 0 1 in
  let last () = seek (Array.length t.rows - 1) (-1) in
  match direction, t.active with
  | (Navigation.Previous | Last), None -> move (last ())
  | (Next | First | Parent | Child), None -> move (first ())
  | First, Some _ -> move (first ())
  | Last, Some _ -> move (last ())
  | Previous, Some id -> move (seek (Map.find_exn t.indices id - 1) (-1))
  | Next, Some id -> move (seek (Map.find_exn t.indices id + 1) 1)
  | Parent, Some id ->
    if is_expanded t id
    then toggle_expanded t tree id
    else move (List.find (old_ancestors t id) ~f:(eligible t))
  | Child, Some id ->
    (match Tree.Node.children (Tree.find tree id |> Option.value_exn) with
     | Leaf -> t
     | Branch { ids; _ } ->
       if not (is_expanded t id)
       then toggle_expanded t tree id
       else move (List.find ids ~f:(eligible t)))
;;

let reveal t tree id ~focus:move_cursor =
  let t = reconcile t tree in
  match Tree.find tree id, Tree.ancestors tree id with
  | Some node, Some ancestors when not (Tree.Node.is_disabled node) ->
    let expanded, changed =
      List.fold
        ancestors
        ~init:(t.expanded, false)
        ~f:(fun (expanded, changed) ancestor ->
          if Map.mem expanded ancestor
          then expanded, changed
          else
            ( Map.set
                expanded
                ~key:ancestor
                ~data:(Tree.Expert.incarnation tree ancestor |> Option.value_exn)
            , true ))
    in
    let t = if changed then rebuild { t with expanded } tree else t in
    if move_cursor then focus t tree id else t
  | None, _ | _, None | Some _, Some _ -> t
;;

let typeahead t tree input =
  let t = reconcile t tree in
  let typeahead, found =
    Tree_typeahead.advance t.typeahead tree ~visible:t.visible ~active:t.active input
  in
  let t = Option.value_map found ~default:t ~f:(fun id -> select t tree id Replace) in
  { t with typeahead }, found
;;
