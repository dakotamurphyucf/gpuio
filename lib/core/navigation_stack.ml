open Core
module Id = Choice.Id

module Entry = struct
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
  let with_label t label = create ~id:t.id ~label t.data
end

type 'a t =
  { entries : 'a Entry.t list
  ; current_index : int
  }

let max_entries = 128
let empty = { entries = []; current_index = -1 }
let singleton entry = { entries = [ entry ]; current_index = 0 }
let entries t = t.entries
let back_entries t = List.take t.entries (Int.max 0 t.current_index)
let current t = if t.current_index < 0 then None else List.nth t.entries t.current_index
let forward_entries t = List.drop t.entries (t.current_index + 1)
let find t id = List.find t.entries ~f:(fun entry -> Id.equal (Entry.id entry) id)
let can_pop t = t.current_index > 0
let can_forward t = t.current_index + 1 < List.length t.entries

let create ?current entries =
  if List.length entries > max_entries
  then Or_error.error_string "navigation history limit exceeded"
  else if
    List.contains_dup entries ~compare:(fun a b -> Id.compare (Entry.id a) (Entry.id b))
  then Or_error.error_string "duplicate navigation entry ID"
  else (
    match current with
    | None -> Ok { entries; current_index = List.length entries - 1 }
    | Some id ->
      (match List.findi entries ~f:(fun _ entry -> Id.equal (Entry.id entry) id) with
       | None -> Or_error.error_string "current navigation entry is absent"
       | Some (current_index, _) -> Ok { entries; current_index }))
;;

let push t entry =
  if Option.is_some (find t (Entry.id entry))
  then Or_error.error_string "navigation push requires a fresh entry ID"
  else create (List.take t.entries (t.current_index + 1) @ [ entry ])
;;

let pop t = if can_pop t then { t with current_index = t.current_index - 1 } else t
let pop_to_root t = if can_pop t then { t with current_index = 0 } else t

let forward t =
  if can_forward t then { t with current_index = t.current_index + 1 } else t
;;

let replace t entry =
  match current t with
  | None -> Ok (singleton entry)
  | Some _ ->
    create
      ~current:(Entry.id entry)
      (List.mapi t.entries ~f:(fun index old ->
         if index = t.current_index then entry else old))
;;

let pop_to t id =
  match List.findi t.entries ~f:(fun _ entry -> Id.equal (Entry.id entry) id) with
  | Some (index, _) when index <= t.current_index -> Ok { t with current_index = index }
  | Some _ | None ->
    Or_error.error_string "navigation destination is not on the back path"
;;

let update t id ~f =
  match find t id with
  | None -> Or_error.error_string "navigation entry is absent"
  | Some entry ->
    let next = Entry.with_data entry (f (Entry.data entry)) in
    Ok
      { t with
        entries =
          List.map t.entries ~f:(fun old ->
            if Id.equal (Entry.id old) id then next else old)
      }
;;

let clear _ = empty
