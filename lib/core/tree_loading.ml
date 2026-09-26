open Core
module Id = Tree.Id
module Boundary = List_paging.Boundary

let max_queued = 64
let max_running = 4
let max_page_nodes = 2048
let max_error_details = 64
let max_error_bytes = 4096

module Stamp = struct
  type t =
    { incarnation : int64
    ; children_revision : int64
    }
  [@@deriving equal]

  let find tree id =
    let open Option.Let_syntax in
    let%bind incarnation = Tree.Expert.incarnation tree id in
    let%map children_revision = Tree.Expert.children_revision tree id in
    { incarnation; children_revision }
  ;;

  let current tree id stamp = Option.exists (find tree id) ~f:(equal stamp)
end

module Request = struct
  type t =
    { owner : unit ref
    ; generation : int64
    ; serial : int64
    ; parent : Id.t
    ; cursor : string option
    ; stamp : Stamp.t
    }

  let parent t = t.parent
  let cursor t = t.cursor
  let generation t = t.generation

  let same a b =
    phys_equal a.owner b.owner
    && Int64.equal a.generation b.generation
    && Int64.equal a.serial b.serial
  ;;
end

module Status = struct
  type t =
    | Ready
    | Queued
    | Loading
    | End
    | Failed of Error.t
  [@@deriving sexp_of]
end

module Completion = struct
  type t =
    | Applied
    | Obsolete
  [@@deriving equal, sexp_of]
end

module Page = struct
  type 'data t =
    { roots : Id.t list
    ; nodes : (Id.t * 'data Tree.Node.t) list
    ; next : Boundary.t
    }
end

module Failure = struct
  type t =
    { stamp : Stamp.t
    ; serial : int64
    }
end

let boundary tree id =
  Option.bind (Tree.find tree id) ~f:(fun node ->
    match Tree.Node.children node with
    | Leaf -> None
    | Branch { next; _ } -> Some next)
;;

module Snapshot = struct
  type 'data t =
    { tree : 'data Tree.t
    ; generation : int64
    ; queued : Request.t list
    ; running : (Id.t, Request.t, Id.comparator_witness) Map.t
    ; failed : (Id.t, Failure.t, Id.comparator_witness) Map.t
    ; errors : (Id.t * Error.t) Int64.Map.t
    ; error_order : int64 list
    }

  let tree t = t.tree
  let generation t = t.generation
  let queued_count t = List.length t.queued
  let running_count t = Map.length t.running
  let failed_count t = Map.length t.failed
  let error_detail_count t = Map.length t.errors

  let status t id =
    Option.map (boundary t.tree id) ~f:(fun boundary ->
      if Map.mem t.running id
      then Status.Loading
      else if List.exists t.queued ~f:(fun request -> Id.equal request.Request.parent id)
      then Queued
      else (
        match Map.find t.failed id with
        | Some failure ->
          Failed
            (Option.value_map
               (Map.find t.errors failure.serial)
               ~f:snd
               ~default:
                 (Error.of_string "Tree load failed; retry to request this branch again"))
        | None ->
          (match boundary with
           | End -> End
           | More _ -> Ready)))
  ;;
end

type 'data t =
  { owner : unit ref
  ; mutable state : 'data Snapshot.t
  ; mutable serial : int64
  ; mutable closed : bool
  }

let empty tree generation =
  { Snapshot.tree
  ; generation
  ; queued = []
  ; running = Map.empty (module Id)
  ; failed = Map.empty (module Id)
  ; errors = Int64.Map.empty
  ; error_order = []
  }
;;

let create tree = { owner = ref (); state = empty tree 0L; serial = 0L; closed = false }
let snapshot t = t.state

let clear_failure state parent =
  match Map.find state.Snapshot.failed parent with
  | None -> state
  | Some failure ->
    { state with
      failed = Map.remove state.failed parent
    ; errors = Map.remove state.errors failure.serial
    ; error_order =
        List.filter state.error_order ~f:(fun serial ->
          not (Int64.equal serial failure.serial))
    }
;;

let enqueue t parent ~retry =
  if t.closed
  then Or_error.error_string "tree loader closed"
  else (
    match Snapshot.status t.state parent with
    | None -> Or_error.error_string "tree load requires a present branch"
    | Some (Loading | Queued | End) -> Ok false
    | Some (Failed _) when not retry -> Ok false
    | Some Ready when retry -> Ok false
    | Some (Ready | Failed _) ->
      if List.length t.state.queued >= max_queued
      then Or_error.error_string "tree load queue limit exceeded"
      else if Int64.equal t.serial Int64.max_value
      then Or_error.error_string "tree load sequence exhausted"
      else (
        let cursor =
          match boundary t.state.tree parent with
          | Some (More cursor) -> cursor
          | None | Some End -> assert false
        in
        let serial = Int64.succ t.serial in
        let request =
          { Request.owner = t.owner
          ; generation = t.state.generation
          ; serial
          ; parent
          ; cursor
          ; stamp = Stamp.find t.state.tree parent |> Option.value_exn
          }
        in
        let state = clear_failure t.state parent in
        t.state <- { state with queued = state.queued @ [ request ] };
        t.serial <- serial;
        Ok true))
;;

let request t parent = enqueue t parent ~retry:false
let retry t parent = enqueue t parent ~retry:true

let take t =
  if t.closed || Map.length t.state.running >= max_running
  then None
  else (
    match t.state.queued with
    | [] -> None
    | request :: queued ->
      t.state
      <- { t.state with
           queued
         ; running = Map.set t.state.running ~key:request.parent ~data:request
         };
      Some request)
;;

let running t = Map.data t.state.running

let is_current t request =
  (not t.closed)
  && phys_equal t.owner request.Request.owner
  && Int64.equal t.state.generation request.generation
  && Stamp.current t.state.tree request.parent request.stamp
  && Option.exists (Map.find t.state.running request.parent) ~f:(Request.same request)
;;

let bounded_error error =
  let message = Error.to_string_hum error in
  if String.is_empty message
  then Error.of_string "Tree load failed"
  else if not (Stdlib.String.is_valid_utf_8 message)
  then Error.of_string "Tree load failed (invalid UTF-8 error message)"
  else (
    let message =
      String.map message ~f:(function
        | '\000' -> '?'
        | c -> c)
    in
    let rec prefix length =
      let candidate = String.prefix message length in
      if Stdlib.String.is_valid_utf_8 candidate then candidate else prefix (length - 1)
    in
    Error.of_string (prefix (Int.min max_error_bytes (String.length message))))
;;

let fail t request error =
  if not (is_current t request)
  then Completion.Obsolete
  else (
    let state = t.state in
    let error_order = state.error_order @ [ request.serial ] in
    let errors =
      Map.set state.errors ~key:request.serial ~data:(request.parent, bounded_error error)
    in
    let error_order, errors =
      if List.length error_order <= max_error_details
      then error_order, errors
      else (
        match error_order with
        | [] -> assert false
        | oldest :: rest -> rest, Map.remove errors oldest)
    in
    t.state
    <- { state with
         running = Map.remove state.running request.parent
       ; failed =
           Map.set
             state.failed
             ~key:request.parent
             ~data:{ Failure.stamp = request.stamp; serial = request.serial }
       ; errors
       ; error_order
       };
    Applied)
;;

let append tree parent (page : _ Page.t) =
  let open Or_error.Let_syntax in
  if List.length page.nodes > max_page_nodes
  then Or_error.error_string "tree page node limit exceeded"
  else if List.exists page.nodes ~f:(fun (id, _) -> Option.is_some (Tree.find tree id))
  then Or_error.error_string "tree page reuses an existing node ID"
  else (
    let%bind (_ : _ Tree.t) = Tree.create ~roots:page.roots page.nodes in
    let node = Tree.find tree parent |> Option.value_exn in
    let children =
      match Tree.Node.children node with
      | Leaf -> assert false
      | Branch { ids; _ } -> ids
    in
    let%bind replacement =
      Tree.Node.create
        ~label:(Tree.Node.label node)
        ~disabled:(Tree.Node.is_disabled node)
        ~children:(Branch { ids = children @ page.roots; next = page.next })
        (Tree.Node.data node)
    in
    let nodes =
      Tree.to_alist tree
      |> List.map ~f:(fun (id, node) ->
        id, if Id.equal id parent then replacement else node)
    in
    Tree.replace tree ~roots:(Tree.roots tree) (nodes @ page.nodes))
;;

let complete t request page =
  if not (is_current t request)
  then Ok Completion.Obsolete
  else (
    let result =
      if
        List.is_empty page.Page.roots
        && List.is_empty page.nodes
        && Boundary.equal page.next (More request.Request.cursor)
      then Or_error.error_string "an empty tree page must advance its cursor or reach end"
      else append t.state.tree request.parent page
    in
    match result with
    | Error error ->
      ignore (fail t request error : Completion.t);
      Error error
    | Ok tree ->
      t.state
      <- { t.state with tree; running = Map.remove t.state.running request.parent };
      Ok Applied)
;;

let cancel t parent =
  if
    Map.mem t.state.running parent
    || List.exists t.state.queued ~f:(fun request ->
      Id.equal request.Request.parent parent)
  then
    t.state
    <- { t.state with
         queued =
           List.filter t.state.queued ~f:(fun request ->
             not (Id.equal request.Request.parent parent))
       ; running = Map.remove t.state.running parent
       }
;;

let cancel_where t ~f =
  let queued =
    List.filter t.state.queued ~f:(fun request -> not (f request.Request.parent))
  in
  let running = Map.filter_keys t.state.running ~f:(fun parent -> not (f parent)) in
  if
    List.length queued <> List.length t.state.queued
    || Map.length running <> Map.length t.state.running
  then t.state <- { t.state with queued; running }
;;

let cancel_subtree t parent =
  cancel_where t ~f:(fun id ->
    Id.equal id parent
    || Option.exists (Tree.ancestors t.state.tree id) ~f:(fun ancestors ->
      List.mem ancestors parent ~equal:Id.equal))
;;

let cancel_hidden t state =
  let state = Tree_state.reconcile state t.state.tree in
  cancel_where t ~f:(fun id ->
    Option.is_none (Tree_state.visible_index state id)
    || not (Tree_state.is_expanded state id))
;;

let invalidate t parent =
  cancel t parent;
  t.state <- clear_failure t.state parent
;;

let update t tree =
  if t.closed
  then Or_error.error_string "tree loader closed"
  else if phys_equal tree t.state.tree
  then Ok ()
  else if Int64.(Tree.revision tree <= Tree.revision t.state.tree)
  then
    Or_error.error_string
      "tree loader update must advance source revision; use reset for a new lineage"
  else (
    let state = t.state in
    if phys_equal (Tree.preorder tree) (Tree.preorder state.tree)
    then t.state <- { state with tree }
    else (
      let valid request = Stamp.current tree request.Request.parent request.stamp in
      let failed =
        Map.filteri state.failed ~f:(fun ~key ~data ->
          Stamp.current tree key data.Failure.stamp)
      in
      let errors =
        Map.filteri state.errors ~f:(fun ~key:serial ~data:(parent, _) ->
          Option.exists (Map.find failed parent) ~f:(fun failure ->
            Int64.equal serial failure.Failure.serial))
      in
      t.state
      <- { state with
           tree
         ; queued = List.filter state.queued ~f:valid
         ; running = Map.filter state.running ~f:valid
         ; failed
         ; errors
         ; error_order = List.filter state.error_order ~f:(Map.mem errors)
         });
    Ok ())
;;

let reset t tree =
  if t.closed
  then Or_error.error_string "tree loader closed"
  else if Int64.equal t.state.generation Int64.max_value
  then Or_error.error_string "tree loader generation exhausted"
  else (
    t.state <- empty tree (Int64.succ t.state.generation);
    Ok ())
;;

let close t =
  if not t.closed
  then (
    t.closed <- true;
    t.state <- empty t.state.tree t.state.generation)
;;
