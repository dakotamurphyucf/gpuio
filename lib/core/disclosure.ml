open Core
module Id = Choice.Id

module Mode = struct
  type t =
    | Single of { allow_empty : bool }
    | Multiple
  [@@deriving equal, sexp_of]
end

module Request = struct
  type t =
    | Expand of Id.t
    | Collapse of Id.t
    | Toggle of Id.t
  [@@deriving equal, sexp_of]
end

type t =
  { items : Choice.Collection.t
  ; mode : Mode.t
  ; expanded : String.Set.t
  ; disabled : bool
  }
[@@deriving equal, sexp_of]

let items t = t.items
let mode t = t.mode
let is_expanded t id = Set.mem t.expanded (Id.to_string id)
let is_disabled t = t.disabled
let with_disabled t disabled = { t with disabled }

let expanded t =
  Choice.Collection.to_list t.items
  |> List.filter_map ~f:(fun item ->
    let id = Choice.id item in
    Option.some_if (is_expanded t id) id)
;;

let create ~items ~mode ~expanded ?(disabled = false) () =
  let open Or_error.Let_syntax in
  let%bind expanded =
    List.fold_result expanded ~init:String.Set.empty ~f:(fun found id ->
      let key = Id.to_string id in
      if Set.mem found key
      then Or_error.error_string "duplicate expanded disclosure ID"
      else if Option.is_none (Choice.Collection.find items id)
      then Or_error.error_string "expanded disclosure item is absent"
      else Ok (Set.add found key))
  in
  let%map () =
    match mode with
    | Mode.Multiple -> Ok ()
    | Single { allow_empty } ->
      if Set.length expanded > 1
      then Or_error.error_string "single disclosure has multiple expanded items"
      else if
        (not allow_empty)
        && (not (List.is_empty (Choice.Collection.to_list items)))
        && Set.is_empty expanded
      then Or_error.error_string "single disclosure requires an expanded item"
      else Ok ()
  in
  { items; mode; expanded; disabled }
;;

let normalize t =
  let expanded = expanded t in
  let expanded =
    match t.mode with
    | Mode.Multiple -> expanded
    | Single { allow_empty } ->
      (match expanded with
       | first :: _ -> [ first ]
       | [] ->
         if allow_empty
         then []
         else (
           let items = Choice.Collection.to_list t.items in
           let fallback =
             match List.find items ~f:(fun item -> not (Choice.is_disabled item)) with
             | Some item -> Some item
             | None -> List.hd items
           in
           Option.to_list (Option.map fallback ~f:Choice.id)))
  in
  { t with expanded = String.Set.of_list (List.map expanded ~f:Id.to_string) }
;;

let with_items t items = normalize { t with items }
let with_mode t mode = normalize { t with mode }

let apply_request t request =
  let id =
    match request with
    | Request.Expand id | Collapse id | Toggle id -> id
  in
  match Choice.Collection.find t.items id with
  | None -> t
  | Some item ->
    if t.disabled || Choice.is_disabled item
    then t
    else (
      let expand =
        match request with
        | Expand _ -> true
        | Collapse _ -> false
        | Toggle _ -> not (is_expanded t id)
      in
      let key = Id.to_string id in
      let expanded =
        match t.mode with
        | Mode.Multiple ->
          if expand then Set.add t.expanded key else Set.remove t.expanded key
        | Single { allow_empty } ->
          if expand
          then String.Set.singleton key
          else if allow_empty
          then Set.remove t.expanded key
          else t.expanded
      in
      { t with expanded })
;;
