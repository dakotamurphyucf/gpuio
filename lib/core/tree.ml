open Core

let max_nodes = 100_000
let max_depth = 128
let max_metadata_bytes = 8 * 1024 * 1024

let validate_text ~name ~max_bytes text =
  if String.is_empty text || String.length text > max_bytes
  then Or_error.errorf "%s must contain 1..%d bytes" name max_bytes
  else if (not (Stdlib.String.is_valid_utf_8 text)) || String.contains text '\000'
  then Or_error.errorf "%s must be UTF-8 without NUL" name
  else Ok ()
;;

module Id = struct
  module T = struct
    type t = string [@@deriving compare, equal, sexp_of]
  end

  include T
  include Comparator.Make (T)

  let of_string text =
    let%map.Or_error () = validate_text ~name:"tree id" ~max_bytes:256 text in
    text
  ;;

  let to_string t = t
end

module Children = struct
  type t =
    | Leaf
    | Branch of
        { ids : Id.t list
        ; next : List_paging.Boundary.t
        }
  [@@deriving equal, sexp_of]

  let ids = function
    | Leaf -> []
    | Branch { ids; _ } -> ids
  ;;

  let cursor = function
    | Leaf | Branch { next = End | More None; _ } -> None
    | Branch { next = More (Some cursor); _ } -> Some cursor
  ;;

  let complete = function
    | Leaf | Branch { next = End; _ } -> true
    | Branch { next = More _; _ } -> false
  ;;
end

module Node = struct
  type 'data t =
    { label : string
    ; disabled : bool
    ; children : Children.t
    ; data : 'data
    ; child_count : int
    ; metadata_bytes : int
    }

  let create ~label ?(disabled = false) ~children data =
    let%bind.Or_error () = validate_text ~name:"tree label" ~max_bytes:4096 label in
    let child_count = List.length (Children.ids children) in
    let cursor_bytes =
      Option.value_map (Children.cursor children) ~default:0 ~f:String.length
    in
    let reference_bytes =
      List.sum
        (module Int)
        (Children.ids children)
        ~f:(fun id -> String.length (Id.to_string id))
    in
    if child_count > max_nodes
    then Or_error.error_string "tree child reference limit exceeded"
    else if cursor_bytes > 4096
    then Or_error.error_string "tree cursor exceeds 4096 bytes"
    else
      Ok
        { label
        ; disabled
        ; children
        ; data
        ; child_count
        ; metadata_bytes = String.length label + cursor_bytes + reference_bytes
        }
  ;;

  let label t = t.label
  let is_disabled t = t.disabled
  let children t = t.children
  let data t = t.data
  let with_data t data = { t with data }
end

module Position = struct
  type t =
    { parent : Id.t option
    ; depth : int
    ; index : int
    ; sibling_count : int option
    }
  [@@deriving equal, sexp_of]
end

module Entry = struct
  type 'data t =
    { node : 'data Node.t
    ; incarnation : int64
    ; children_revision : int64
    }
end

type 'data t =
  { roots : Id.t list
  ; entries : (Id.t, 'data Entry.t, Id.comparator_witness) Map.t
  ; positions : (Id.t, Position.t, Id.comparator_witness) Map.t
  ; preorder : Id.t list
  ; metadata_bytes : int
  ; revision : int64
  }

let assign entries positions ~parent ~complete ids =
  let sibling_count = if complete then Some (List.length ids) else None in
  List.foldi ids ~init:(Ok positions) ~f:(fun index positions id ->
    let%bind.Or_error positions = positions in
    if not (Map.mem entries id)
    then Or_error.errorf "tree references missing node %s" (Id.to_string id)
    else if Map.mem positions id
    then
      Or_error.errorf
        "tree node %s has more than one root/parent reference"
        (Id.to_string id)
    else
      Ok
        (Map.set
           positions
           ~key:id
           ~data:{ Position.parent; depth = 0; index; sibling_count }))
;;

let topology entries roots =
  let open Or_error.Let_syntax in
  let%bind positions =
    assign entries (Map.empty (module Id)) ~parent:None ~complete:true roots
  in
  let%bind positions =
    Map.fold entries ~init:(Ok positions) ~f:(fun ~key ~data positions ->
      let%bind positions = positions in
      let children = data.Entry.node.children in
      assign
        entries
        positions
        ~parent:(Some key)
        ~complete:(Children.complete children)
        (Children.ids children))
  in
  let rec walk stack positions visited reversed =
    match stack with
    | [] ->
      if Set.length visited <> Map.length entries
      then Or_error.error_string "tree contains unreachable nodes or a cycle"
      else Ok (positions, List.rev reversed)
    | (id, depth) :: rest ->
      if depth > max_depth
      then Or_error.error_string "tree depth limit exceeded"
      else if Set.mem visited id
      then Or_error.error_string "tree contains a cycle"
      else (
        let node = (Map.find_exn entries id).Entry.node in
        let children =
          List.map (Children.ids node.children) ~f:(fun child -> child, depth + 1)
        in
        let position = { (Map.find_exn positions id) with Position.depth } in
        walk
          (children @ rest)
          (Map.set positions ~key:id ~data:position)
          (Set.add visited id)
          (id :: reversed))
  in
  walk (List.map roots ~f:(fun root -> root, 1)) positions (Set.empty (module Id)) []
;;

let admit ~revision ~previous ~roots nodes =
  let open Or_error.Let_syntax in
  let count = List.length nodes in
  if count > max_nodes
  then Or_error.error_string "tree node limit exceeded"
  else (
    let root_bytes =
      List.sum (module Int) roots ~f:(fun id -> String.length (Id.to_string id))
    in
    let%bind metadata_bytes, references =
      List.fold
        nodes
        ~init:(Ok (root_bytes, List.length roots))
        ~f:(fun totals (id, node) ->
          let%bind bytes, references = totals in
          let bytes =
            bytes + String.length (Id.to_string id) + node.Node.metadata_bytes
          in
          let references = references + node.child_count in
          if bytes > max_metadata_bytes
          then Or_error.error_string "tree metadata limit exceeded"
          else if references > count
          then Or_error.error_string "tree has more root/child references than nodes"
          else Ok (bytes, references))
    in
    if references <> count
    then Or_error.error_string "tree has unreferenced nodes"
    else (
      let%bind entries =
        match Map.of_alist (module Id) nodes with
        | `Duplicate_key _ -> Or_error.error_string "tree node IDs must be unique"
        | `Ok nodes ->
          Ok
            (Map.mapi nodes ~f:(fun ~key ~data:node ->
               match Map.find previous key with
               | None ->
                 { Entry.node; incarnation = revision; children_revision = revision }
               | Some (old : _ Entry.t) ->
                 { Entry.node
                 ; incarnation = old.incarnation
                 ; children_revision =
                     (if Children.equal old.node.children node.children
                      then old.children_revision
                      else revision)
                 }))
      in
      let%map positions, preorder = topology entries roots in
      { roots; entries; positions; preorder; metadata_bytes; revision }))
;;

let create ~roots nodes =
  admit ~revision:0L ~previous:(Map.empty (module Id)) ~roots nodes
;;

let revision t = t.revision
let length t = Map.length t.entries
let metadata_bytes t = t.metadata_bytes
let roots t = t.roots
let preorder t = t.preorder
let find t id = Map.find t.entries id |> Option.map ~f:(fun entry -> entry.Entry.node)
let position t id = Map.find t.positions id

let ancestors t id =
  if not (Map.mem t.entries id)
  then None
  else (
    let rec collect id parents =
      match (Map.find_exn t.positions id).Position.parent with
      | None -> parents
      | Some parent -> collect parent (parent :: parents)
    in
    Some (collect id []))
;;

let to_alist t =
  List.map t.preorder ~f:(fun id -> id, (Map.find_exn t.entries id).Entry.node)
;;

let next_revision t =
  if Int64.equal t.revision Int64.max_value
  then Or_error.error_string "tree revision exhausted"
  else Ok (Int64.succ t.revision)
;;

let set_data t ~id data =
  let open Or_error.Let_syntax in
  match Map.find t.entries id with
  | None -> Or_error.error_string "cannot update an absent tree node"
  | Some entry ->
    let%map revision = next_revision t in
    let entry = { entry with Entry.node = Node.with_data entry.node data } in
    { t with entries = Map.set t.entries ~key:id ~data:entry; revision }
;;

let replace t ~roots nodes =
  let%bind.Or_error revision = next_revision t in
  admit ~revision ~previous:t.entries ~roots nodes
;;

let fold_changed_nodes t ~previous ~init ~f =
  Map.fold_symmetric_diff
    previous.entries
    t.entries
    ~data_equal:(fun a b -> phys_equal a.Entry.node b.Entry.node)
    ~init
    ~f:(fun acc (id, _) -> f acc id)
;;

module Expert = struct
  let incarnation t id =
    Map.find t.entries id |> Option.map ~f:(fun entry -> entry.Entry.incarnation)
  ;;

  let children_revision t id =
    Map.find t.entries id |> Option.map ~f:(fun entry -> entry.Entry.children_revision)
  ;;
end
