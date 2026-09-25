open Core
module Id = Choice.Id

module Tab = struct
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
  { tabs : 'a Tab.t list
  ; active : Id.t option
  }

let max_tabs = 128
let empty = { tabs = []; active = None }
let tabs t = t.tabs
let find t id = List.find t.tabs ~f:(fun tab -> Id.equal (Tab.id tab) id)
let active t = Option.bind t.active ~f:(find t)

let create ?active tabs =
  if List.length tabs > max_tabs
  then Or_error.error_string "workspace tab limit exceeded"
  else if List.contains_dup tabs ~compare:(fun a b -> Id.compare (Tab.id a) (Tab.id b))
  then Or_error.error_string "duplicate workspace tab ID"
  else (
    let active =
      match active with
      | Some _ -> active
      | None -> Option.map (List.hd tabs) ~f:Tab.id
    in
    let t = { tabs; active } in
    if Option.for_all active ~f:(fun id -> Option.is_some (find t id))
    then Ok t
    else Or_error.error_string "active tab is absent")
;;

let select t id =
  match find t id with
  | None -> Or_error.error_string "tab is absent"
  | Some _ -> Ok { t with active = Some id }
;;

let neighbor t delta =
  match t.active with
  | None -> t
  | Some id ->
    let index =
      List.findi t.tabs ~f:(fun _ tab -> Id.equal (Tab.id tab) id)
      |> Option.value_exn
      |> fst
    in
    let index = (index + delta + List.length t.tabs) mod List.length t.tabs in
    { t with active = Some (List.nth_exn t.tabs index |> Tab.id) }
;;

let next t = neighbor t 1
let previous t = neighbor t (-1)

let add t ?(activate = true) tab =
  let active = if activate then Some (Tab.id tab) else t.active in
  create ?active (t.tabs @ [ tab ])
;;

let remove t id =
  match List.findi t.tabs ~f:(fun _ tab -> Id.equal (Tab.id tab) id) with
  | None -> Or_error.error_string "tab is absent"
  | Some (index, tab) ->
    let tabs = List.filter t.tabs ~f:(fun tab -> not (Id.equal (Tab.id tab) id)) in
    let active =
      if Option.equal Id.equal t.active (Some id)
      then
        if List.is_empty tabs
        then None
        else Option.map (List.nth tabs (Int.min index (List.length tabs - 1))) ~f:Tab.id
      else t.active
    in
    Ok ({ tabs; active }, tab)
;;

let move t id ~index =
  if index < 0 || index >= List.length t.tabs
  then Or_error.error_string "invalid tab destination"
  else (
    match find t id with
    | None -> Or_error.error_string "tab is absent"
    | Some tab ->
      let rest = List.filter t.tabs ~f:(fun tab -> not (Id.equal (Tab.id tab) id)) in
      let before, after = List.split_n rest index in
      Ok { t with tabs = before @ (tab :: after) })
;;

let replace t id ~f =
  match find t id with
  | None -> Or_error.error_string "tab is absent"
  | Some old ->
    let%map.Or_error tab = f old in
    { t with
      tabs =
        List.map t.tabs ~f:(fun current ->
          if Id.equal (Tab.id current) id then tab else current)
    }
;;

let update t id ~f =
  replace t id ~f:(fun tab -> Ok (Tab.with_data tab (f (Tab.data tab))))
;;

let rename t id ~label = replace t id ~f:(fun tab -> Tab.with_label tab label)

let choices t ~label =
  let open Or_error.Let_syntax in
  let%bind items =
    List.map t.tabs ~f:(fun tab ->
      Choice.create ~id:(Tab.id tab) ~label:(Tab.label tab) ())
    |> Or_error.all
  in
  let%bind options = Choice.Collection.create items in
  Choice.Config.create ~label ~options ~selected:t.active ()
;;
