open Core
module Direction = List_paging.Direction
module Boundary = List_paging.Boundary
module Status = List_paging.Status
module Completion = List_paging.Completion

let max_page_rows = 2048
let max_cursor_bytes = 4096
let max_error_bytes = 4096

module Request = struct
  type 'query t =
    { owner : unit ref
    ; query : 'query
    ; generation : int64
    ; serial : int64
    ; direction : Direction.t
    ; cursor : string option
    }

  let query t = t.query
  let generation t = t.generation
  let direction t = t.direction
  let cursor t = t.cursor
end

module Snapshot = struct
  type ('query, 'data) t =
    { query : 'query
    ; data : 'data Table_data.t
    ; generation : int64
    ; before : Status.t
    ; after : Status.t
    }
end

module Edge = struct
  type 'query t =
    | Ready of string option
    | Loading of 'query Request.t
    | End
    | Failed of string option * Error.t

  let of_boundary : Boundary.t -> _ t = function
    | End -> End
    | More cursor -> Ready cursor
  ;;

  let status : _ t -> Status.t = function
    | Ready _ -> Ready
    | Loading _ -> Loading
    | End -> End
    | Failed (_, error) -> Failed error
  ;;
end

type ('query, 'data) t =
  { owner : unit ref
  ; mutable query : 'query
  ; mutable generation : int64
  ; mutable serial : int64
  ; mutable data : 'data Table_data.t
  ; mutable before : 'query Edge.t
  ; mutable after : 'query Edge.t
  }

let validate_boundary : Boundary.t -> unit Or_error.t = function
  | End | More None -> Ok ()
  | More (Some cursor) ->
    if String.length cursor > max_cursor_bytes
    then Or_error.errorf "table cursor exceeds %d bytes" max_cursor_bytes
    else Ok ()
;;

let create ~query data ~before ~after =
  let%bind.Or_error () = validate_boundary before in
  let%map.Or_error () = validate_boundary after in
  { owner = ref ()
  ; query
  ; generation = 0L
  ; serial = 0L
  ; data
  ; before = Edge.of_boundary before
  ; after = Edge.of_boundary after
  }
;;

let data t = t.data
let query t = t.query
let generation t = t.generation

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

let snapshot t =
  { Snapshot.query = t.query
  ; data = t.data
  ; generation = t.generation
  ; before = status t Before
  ; after = status t After
  }
;;

let start t direction ~retry =
  let cursor =
    match edge t direction with
    | Edge.Ready cursor when not retry -> Some cursor
    | Failed (cursor, _) when retry -> Some cursor
    | Ready _ | Failed _ | Loading _ | End -> None
  in
  match cursor with
  | None -> Ok None
  | Some cursor ->
    if Int64.equal t.serial Int64.max_value
    then Or_error.error_string "table paging request sequence exhausted"
    else (
      t.serial <- Int64.succ t.serial;
      let request : _ Request.t =
        { owner = t.owner
        ; query = t.query
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

let is_current t (request : _ Request.t) =
  phys_equal t.owner request.owner
  && Int64.equal t.generation request.generation
  &&
  match edge t request.direction with
  | Loading active -> Int64.equal active.serial request.serial
  | Ready _ | Failed _ | End -> false
;;

let bounded_error error =
  let message = Error.to_string_hum error in
  if String.is_empty message
  then Error.of_string "Table load failed"
  else if not (Stdlib.String.is_valid_utf_8 message)
  then Error.of_string "Table load failed (invalid UTF-8 error message)"
  else (
    let message =
      String.map message ~f:(fun c -> if Char.equal c '\000' then '?' else c)
    in
    let rec prefix length =
      let candidate = String.prefix message length in
      if Stdlib.String.is_valid_utf_8 candidate then candidate else prefix (length - 1)
    in
    Error.of_string (prefix (Int.min max_error_bytes (String.length message))))
;;

let fail t (request : _ Request.t) error =
  if not (is_current t request)
  then Completion.Obsolete
  else (
    set_edge t request.direction (Failed (request.cursor, bounded_error error));
    Applied)
;;

let validate_page rows =
  if List.length rows > max_page_rows
  then Or_error.errorf "table page exceeds %d rows" max_page_rows
  else Ok ()
;;

let complete t (request : _ Request.t) ~rows ~next =
  if not (is_current t request)
  then Ok Completion.Obsolete
  else (
    let result =
      let%bind.Or_error () = validate_page rows in
      let%bind.Or_error () = validate_boundary next in
      if List.is_empty rows && Boundary.equal next (More request.cursor)
      then
        Or_error.error_string "an empty table page must advance its cursor or reach end"
      else (
        let at =
          match request.direction with
          | Before -> 0
          | After -> Table_data.length t.data
        in
        Table_data.splice t.data ~at ~remove:0 rows)
    in
    match result with
    | Error error ->
      ignore (fail t request error : Completion.t);
      Error error
    | Ok data ->
      t.data <- data;
      set_edge t request.direction (Edge.of_boundary next);
      Ok Completion.Applied)
;;

let cancel t (request : _ Request.t) =
  if not (is_current t request)
  then Completion.Obsolete
  else (
    set_edge t request.direction (Ready request.cursor);
    Applied)
;;

let reset t ~query data ~before ~after =
  let%bind.Or_error () = validate_boundary before in
  let%bind.Or_error () = validate_boundary after in
  if Int64.equal t.generation Int64.max_value
  then Or_error.error_string "table paging generation exhausted"
  else (
    t.query <- query;
    t.generation <- Int64.succ t.generation;
    t.data <- data;
    t.before <- Edge.of_boundary before;
    t.after <- Edge.of_boundary after;
    Ok ())
;;

let set t ~key ~data =
  let%map.Or_error data = Table_data.set t.data ~key ~data in
  t.data <- data
;;

let append t rows =
  match t.after with
  | Ready _ | Loading _ | Failed _ ->
    Or_error.error_string "table append requires a known latest boundary"
  | End ->
    let%bind.Or_error () = validate_page rows in
    let%map.Or_error data =
      Table_data.splice t.data ~at:(Table_data.length t.data) ~remove:0 rows
    in
    t.data <- data
;;

module Expert = struct
  let is_current = is_current
end
