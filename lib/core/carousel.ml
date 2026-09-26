open Core
module Id = Choice.Id

module Item = struct
  type 'a t =
    { id : Id.t
    ; label : string
    ; data : 'a
    }

  let create ~id ~label data =
    let%map.Or_error (_ : Choice.t) = Choice.create ~id ~label () in
    { id; label; data }
  ;;

  let id t = t.id
  let label t = t.label
  let data t = t.data
  let with_data t data = { t with data }
end

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Auto_advance = struct
  type t = { interval_ms : int64 } [@@deriving equal, sexp_of]

  let create ?(interval = Time_ns.Span.of_sec 5.) () =
    if Time_ns.Span.(interval < of_sec 1. || interval > of_hr 1.)
    then Or_error.error_string "carousel interval must be in 1 second..1 hour"
    else (
      let ns = Time_ns.Span.to_int63_ns interval |> Int63.to_int64 in
      Ok { interval_ms = Int64.((ns + 999_999L) / 1_000_000L) })
  ;;

  let interval t =
    Time_ns.Span.of_int63_ns Int63.(of_int64_exn t.interval_ms * of_int 1_000_000)
  ;;
end

module Request = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Select of Id.t
    | Auto_next of
        { revision : int64
        ; from : Id.t
        ; target : Id.t
        }
  [@@deriving equal, sexp_of]

  let previous = Previous
  let next = Next
  let first = First
  let last = Last
  let select id = Select id
end

type 'a t =
  { items : 'a Item.t list
  ; index : int
  ; looping : bool
  ; disabled : bool
  ; auto_advance : Auto_advance.t option
  ; revision : int64
  ; direction : Gpuio_protocol.Carousel_wire.Direction.t
  }

let max_items = 128
let max_metadata_bytes = 256 * 1024
let items t = t.items
let selected t = if t.index < 0 then None else List.nth t.items t.index
let find t id = List.find t.items ~f:(fun item -> Id.equal (Item.id item) id)
let is_looping t = t.looping
let is_disabled t = t.disabled
let auto_advance t = t.auto_advance
let ids items = List.map items ~f:Item.id

let validate items =
  if List.length items > max_items
  then Or_error.error_string "carousel item limit exceeded"
  else if List.contains_dup (ids items) ~compare:Id.compare
  then Or_error.error_string "duplicate carousel item ID"
  else if
    List.sum
      (module Int)
      items
      ~f:(fun item ->
        String.length (Id.to_string (Item.id item)) + String.length (Item.label item))
    > max_metadata_bytes
  then Or_error.error_string "carousel metadata limit exceeded"
  else Ok ()
;;

let index_of items id =
  List.findi items ~f:(fun _ item -> Id.equal (Item.id item) id) |> Option.map ~f:fst
;;

let create ?selected ?(looping = false) ?(disabled = false) ?auto_advance items =
  let%bind.Or_error () = validate items in
  let%map.Or_error index =
    match selected with
    | None -> Ok (if List.is_empty items then -1 else 0)
    | Some id ->
      index_of items id
      |> Or_error.of_option ~error:(Error.of_string "selected carousel item is absent")
  in
  { items; index; looping; disabled; auto_advance; revision = 0L; direction = Direct }
;;

let bump t =
  if Int64.equal t.revision Int64.max_value
  then Or_error.error_string "carousel revision exhausted"
  else Ok { t with revision = Int64.succ t.revision }
;;

let can_previous t =
  (not t.disabled) && List.length t.items > 1 && (t.looping || t.index > 0)
;;

let can_next t =
  (not t.disabled)
  && List.length t.items > 1
  && (t.looping || t.index + 1 < List.length t.items)
;;

let with_items t items =
  let%bind.Or_error () = validate items in
  let current = selected t |> Option.map ~f:Item.id in
  let index =
    Option.bind current ~f:(index_of items)
    |> Option.value ~default:(Int.min (Int.max 0 t.index) (List.length items - 1))
  in
  let next = { t with items; index } in
  if List.equal Id.equal (ids t.items) (ids items)
  then Ok next
  else bump { next with direction = Direct }
;;

let with_looping t looping =
  if Bool.equal t.looping looping then Ok t else bump { t with looping }
;;

let with_disabled t disabled =
  if Bool.equal t.disabled disabled then Ok t else bump { t with disabled }
;;

let with_auto_advance t auto_advance =
  if Option.equal Auto_advance.equal t.auto_advance auto_advance
  then Ok t
  else bump { t with auto_advance }
;;

let set_index t ?(direction = Gpuio_protocol.Carousel_wire.Direction.Direct) index =
  if index = t.index then Ok t else bump { t with index; direction }
;;

let select t id =
  match index_of t.items id with
  | None -> Or_error.error_string "selected carousel item is absent"
  | Some index -> set_index t index
;;

let next_index t = if can_next t then (t.index + 1) % List.length t.items else t.index

let previous_index t =
  if can_previous t
  then (t.index + List.length t.items - 1) % List.length t.items
  else t.index
;;

let apply_request t (request : Request.t) =
  if t.disabled || t.index < 0
  then Ok t
  else (
    match request with
    | Previous -> set_index t ~direction:Previous (previous_index t)
    | Next -> set_index t ~direction:Next (next_index t)
    | First -> set_index t 0
    | Last -> set_index t (List.length t.items - 1)
    | Select id ->
      (match index_of t.items id with
       | None -> Ok t
       | Some index -> set_index t index)
    | Auto_next { revision; from; target } ->
      if
        (not (Int64.equal revision t.revision))
        || Option.is_none t.auto_advance
        || not (can_next t)
      then Ok t
      else (
        let current = List.nth_exn t.items t.index in
        let next = next_index t in
        if
          Id.equal from (Item.id current)
          && Id.equal target (Item.id (List.nth_exn t.items next))
        then set_index t ~direction:Next next
        else Ok t))
;;

let restart_auto_advance = bump

module Expert = struct
  let to_wire t ~axis : Gpuio_protocol.Carousel_wire.Config.t =
    { revision = t.revision
    ; ids = List.map t.items ~f:(fun item -> Id.to_string (Item.id item))
    ; selected = (if t.index < 0 then None else Some (Int64.of_int t.index))
    ; looping = t.looping
    ; disabled = t.disabled
    ; axis =
        (match axis with
         | Axis.Horizontal -> Horizontal
         | Vertical -> Vertical)
    ; auto_advance_ms = Option.map t.auto_advance ~f:(fun t -> t.Auto_advance.interval_ms)
    ; direction = t.direction
    }
  ;;

  let request_of_wire (request : Gpuio_protocol.Carousel_wire.Request.t) =
    if not (Gpuio_protocol.Carousel_wire.Request.valid request)
    then Or_error.error_string "invalid carousel request"
    else (
      match request with
      | Previous -> Ok Request.Previous
      | Next -> Ok Request.Next
      | First -> Ok Request.First
      | Last -> Ok Request.Last
      | Select id -> Id.of_string id |> Or_error.map ~f:Request.select
      | Auto_next { revision; from; target } ->
        let%bind.Or_error from = Id.of_string from in
        let%map.Or_error target = Id.of_string target in
        Request.Auto_next { revision; from; target })
  ;;
end
