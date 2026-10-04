open Core

let max_items = 4096
let max_groups = 256
let max_text_bytes = 262144
let max_config_bytes = 786432
let max_query_bytes = 262144
let max_event_bytes = max_query_bytes + 512
let max_slots = max_items + max_groups + 4
let max_presentation_bytes = 1048576

let text ~empty ~limit value =
  (empty || not (String.is_empty value))
  && String.length value <= limit
  && Stdlib.String.is_valid_utf_8 value
  && not (String.contains value '\000')
;;

module Item = struct
  type t =
    { id : string
    ; label : string
    ; disabled : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Group = struct
  type t =
    { id : string
    ; label : string
    ; items : Item.t list
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Collection = struct
  type t =
    | Flat of Item.t list
    | Grouped of Group.t list
  [@@deriving bin_io, equal, sexp_of]
end

module Selection = struct
  type t =
    | Single of string option
    | Multiple of string list
  [@@deriving bin_io, equal, sexp_of]
end

module Search = struct
  type t =
    | None
    | Substring
    | Application
  [@@deriving bin_io, equal, sexp_of]
end

module Open_state = struct
  type t =
    | Managed of bool
    | Controlled of bool
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; options : Collection.t
    ; selected : Selection.t
    ; disabled : bool
    ; search : Search.t
    ; clearable : bool
    ; open_state : Open_state.t
    ; placeholder : string
    ; search_placeholder : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let add_items (ids, count, bytes) items =
      if List.length items > max_items - count
      then None
      else
        List.fold_result
          items
          ~init:(ids, count, bytes)
          ~f:(fun (ids, count, bytes) item ->
            let bytes = bytes + String.length item.Item.id + String.length item.label in
            if
              bytes > max_text_bytes
              || (not (text ~empty:false ~limit:256 item.id))
              || (not (text ~empty:false ~limit:4096 item.label))
              || Set.mem ids item.id
            then Error ()
            else Ok (Set.add ids item.id, count + 1, bytes))
        |> Result.ok
    in
    let catalog =
      match t.options with
      | Collection.Flat items -> add_items (String.Set.empty, 0, 0) items
      | Grouped groups ->
        if List.length groups > max_groups
        then None
        else
          List.fold_result
            groups
            ~init:(String.Set.empty, (String.Set.empty, 0, 0))
            ~f:(fun (groups, (ids, count, bytes)) group ->
              let bytes =
                bytes + String.length group.Group.id + String.length group.label
              in
              if
                bytes > max_text_bytes
                || Set.mem groups group.id
                || (not (text ~empty:false ~limit:256 group.id))
                || not (text ~empty:false ~limit:1024 group.label)
              then Error ()
              else (
                match add_items (ids, count, bytes) group.items with
                | None -> Error ()
                | Some state -> Ok (Set.add groups group.id, state)))
          |> Result.ok
          |> Option.map ~f:snd
    in
    text ~empty:false ~limit:1024 t.label
    && text ~empty:true ~limit:1024 t.placeholder
    && text ~empty:true ~limit:1024 t.search_placeholder
    && Option.value_map catalog ~default:false ~f:(fun (ids, _, _) ->
      let selected =
        match t.selected with
        | Single id -> Option.to_list id
        | Multiple ids -> ids
      in
      List.length selected <= max_items
      && List.for_all selected ~f:(Set.mem ids)
      && Set.length (String.Set.of_list selected) = List.length selected)
  ;;
end

module Request = struct
  type t =
    | Select of string
    | Toggle of string
    | Clear
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Select id | Toggle id -> text ~empty:false ~limit:256 id
    | Clear -> true
  ;;
end

module Open_reason = struct
  type t =
    | Trigger
    | Keyboard
    | Escape
    | Outside_pointer
    | Focus_left
    | Selection
  [@@deriving bin_io, equal, sexp_of]

  let allows t open_ =
    match t with
    | Trigger -> true
    | Keyboard -> open_
    | Escape | Outside_pointer | Focus_left | Selection -> not open_
  ;;
end

module Visibility_reason = struct
  type t =
    | Interaction of Open_reason.t
    | Application
    | Unavailable
  [@@deriving bin_io, equal, sexp_of]

  let allows t open_ =
    match t with
    | Interaction reason -> Open_reason.allows reason open_
    | Application -> true
    | Unavailable -> not open_
  ;;
end

module Visibility = struct
  type t =
    | Snapshot of bool
    | Changed of bool * Visibility_reason.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Snapshot _ -> true
    | Changed (open_, reason) -> Visibility_reason.allows reason open_
  ;;
end

module Query = struct
  type t =
    { node : Node_id.t
    ; snapshot : Editor_wire.Snapshot.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let s = t.snapshot in
    let boundary offset =
      Int64.(offset >= 0L && offset <= of_int (String.length s.text))
      &&
      let offset = Int64.to_int_exn offset in
      offset = String.length s.text || Char.to_int s.text.[offset] land 0xc0 <> 0x80
    in
    let range (r : Editor_wire.Selection.t) = boundary r.anchor && boundary r.head in
    Int64.(s.revision >= 0L)
    && text ~empty:true ~limit:max_query_bytes s.text
    && (not (String.exists s.text ~f:(fun c -> Char.equal c '\n' || Char.equal c '\r')))
    && range s.selection
    && Option.value_map s.composition ~default:true ~f:(fun r ->
      Int64.(r.anchor <= r.head) && range r)
  ;;
end

module Event = struct
  type t =
    | Selection_requested of Request.t * Query.t option
    | Open_requested of bool * Open_reason.t
    | Visibility of Visibility.t
    | Query_changed of Query.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Selection_requested (request, query) ->
      Request.valid request
      && Option.value_map query ~default:true ~f:(fun query ->
        Query.valid query && Option.is_none query.snapshot.composition)
    | Open_requested (open_, reason) -> Open_reason.allows reason open_
    | Visibility value -> Visibility.valid value
    | Query_changed query -> Query.valid query
  ;;
end

module Checkmark = struct
  type t =
    | Native
    | Custom
  [@@deriving bin_io, equal, sexp_of]
end

module Slot = struct
  type t =
    | Trigger
    | Query
    | Empty
    | Footer
    | Group of string
    | Option of string * Checkmark.t
  [@@deriving bin_io, equal, sexp_of]
end

let valid_slots (config : Config.t) slots =
  if List.length slots > max_slots
  then false
  else (
    let groups, items =
      match config.options with
      | Flat items -> [], items
      | Grouped groups ->
        groups, List.concat_map groups ~f:(fun group -> group.Group.items)
    in
    let group_ids =
      List.map groups ~f:(fun group -> group.Group.id) |> String.Set.of_list
    in
    let item_ids = List.map items ~f:(fun item -> item.Item.id) |> String.Set.of_list in
    List.fold_result
      slots
      ~init:(String.Set.empty, String.Set.empty, Int.Set.empty)
      ~f:(fun (groups, items, fixed) slot ->
        let fixed_slot tag =
          if Set.mem fixed tag then Error () else Ok (groups, items, Set.add fixed tag)
        in
        match slot with
        | Slot.Trigger -> fixed_slot 0
        | Query -> fixed_slot 1
        | Empty -> fixed_slot 2
        | Footer -> fixed_slot 3
        | Group id ->
          if Set.mem groups id || not (Set.mem group_ids id)
          then Error ()
          else Ok (Set.add groups id, items, fixed)
        | Option (id, _) ->
          if Set.mem items id || not (Set.mem item_ids id)
          then Error ()
          else Ok (groups, Set.add items id, fixed))
    |> Result.ok
    |> Option.value_map ~default:false ~f:(fun (_, _, fixed) ->
      Bool.equal (Set.mem fixed 1) (not (Search.equal config.search None))))
;;
