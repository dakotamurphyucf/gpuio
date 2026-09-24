open Core

module Direction = struct
  type t =
    | Before
    | After
  [@@deriving equal, sexp_of]
end

module Boundary = struct
  type t =
    | End
    | More of string option
  [@@deriving equal, sexp_of]
end

module Request = struct
  type t =
    { owner : unit ref
    ; generation : int64
    ; serial : int64
    ; direction : Direction.t
    ; cursor : string option
    }

  let direction t = t.direction
  let cursor t = t.cursor
  let generation t = t.generation
end

module Status = struct
  type t =
    | Ready
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

module Edge = struct
  type t =
    | Ready of string option
    | Loading of Request.t
    | End
    | Failed of string option * Error.t

  let of_boundary : Boundary.t -> t = function
    | End -> End
    | More cursor -> Ready cursor
  ;;

  let status : t -> Status.t = function
    | Ready _ -> Ready
    | Loading _ -> Loading
    | End -> End
    | Failed (_, error) -> Failed error
  ;;
end

type ('key, 'data, 'cmp) t =
  { owner : unit ref
  ; mutable generation : int64
  ; mutable serial : int64
  ; mutable items : ('key, 'data, 'cmp) List_collection.t
  ; mutable before : Edge.t
  ; mutable after : Edge.t
  }

let create items ~before ~after =
  { owner = ref ()
  ; generation = 0L
  ; serial = 0L
  ; items
  ; before = Edge.of_boundary before
  ; after = Edge.of_boundary after
  }
;;

let items t = t.items

let edge t = function
  | Direction.Before -> t.before
  | After -> t.after
;;

let set_edge t direction value =
  match direction with
  | Direction.Before -> t.before <- value
  | After -> t.after <- value
;;

let status t direction = Edge.status (edge t direction)

let start t direction ~retry =
  let cursor =
    match edge t direction with
    | Ready cursor when not retry -> Some cursor
    | Failed (cursor, _) when retry -> Some cursor
    | Ready _ | Failed _ | Loading _ | End -> None
  in
  match cursor with
  | None -> Ok None
  | Some cursor ->
    if Int64.equal t.serial Int64.max_value
    then Or_error.error_string "list paging request sequence exhausted"
    else (
      t.serial <- Int64.succ t.serial;
      let request : Request.t =
        { owner = t.owner
        ; generation = t.generation
        ; serial = t.serial
        ; direction
        ; cursor
        }
      in
      set_edge t direction (Loading request);
      Ok (Some request))
;;

let request t direction = start t direction ~retry:false
let retry t direction = start t direction ~retry:true

let is_current t (request : Request.t) =
  phys_equal t.owner request.owner
  && Int64.equal t.generation request.generation
  &&
  match edge t request.direction with
  | Loading active -> Int64.equal active.serial request.serial
  | Ready _ | Failed _ | End -> false
;;

let fail t (request : Request.t) error =
  if not (is_current t request)
  then Completion.Obsolete
  else (
    set_edge t request.direction (Failed (request.cursor, error));
    Applied)
;;

let complete t (request : Request.t) ~rows ~next =
  if not (is_current t request)
  then Ok Completion.Obsolete
  else (
    let result =
      if List.is_empty rows && Boundary.equal next (More request.cursor)
      then Or_error.error_string "an empty list page must advance its cursor or reach end"
      else (
        let at =
          match request.direction with
          | Before -> 0
          | After -> List_collection.length t.items
        in
        List_collection.splice t.items ~at ~remove:0 rows)
    in
    match result with
    | Error error ->
      ignore (fail t request error : Completion.t);
      Error error
    | Ok items ->
      t.items <- items;
      set_edge t request.direction (Edge.of_boundary next);
      Ok Applied)
;;

let cancel t (request : Request.t) =
  if not (is_current t request)
  then Completion.Obsolete
  else (
    set_edge t request.direction (Ready request.cursor);
    Applied)
;;

let reset t items ~before ~after =
  if Int64.equal t.generation Int64.max_value
  then Or_error.error_string "list paging generation exhausted"
  else (
    t.generation <- Int64.succ t.generation;
    t.items <- items;
    t.before <- Edge.of_boundary before;
    t.after <- Edge.of_boundary after;
    Ok ())
;;

let set t ~key ~data =
  let open Or_error.Let_syntax in
  let%map items = List_collection.set t.items ~key ~data in
  t.items <- items
;;
