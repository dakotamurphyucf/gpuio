open Core

let validate_text ~name ~max_bytes text =
  if String.is_empty text || String.length text > max_bytes
  then Or_error.errorf "%s must contain 1..%d bytes" name max_bytes
  else if (not (Stdlib.String.is_valid_utf_8 text)) || String.contains text '\000'
  then Or_error.errorf "%s must be UTF-8 without NUL" name
  else Ok ()
;;

module Id = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string text =
    let%map.Or_error () = validate_text ~name:"choice id" ~max_bytes:256 text in
    text
  ;;

  let to_string t = t
end

type t =
  { id : Id.t
  ; label : string
  ; disabled : bool
  }
[@@deriving equal, sexp_of]

let create ~id ~label ?(disabled = false) () =
  let%map.Or_error () = validate_text ~name:"choice label" ~max_bytes:4096 label in
  { id; label; disabled }
;;

let id t = t.id
let label t = t.label
let is_disabled t = t.disabled

module Collection = struct
  type item = t [@@deriving equal, sexp_of]

  type t =
    { items : item list
    ; by_id : item String.Map.t
    }

  let equal left right = List.equal equal_item left.items right.items
  let sexp_of_t t = [%sexp (t.items : item list)]
  let max_choices = 4096
  let max_text_bytes = 262_144

  let create items =
    if List.length items > max_choices
    then Or_error.errorf "choice collection exceeds %d items" max_choices
    else (
      let%map.Or_error by_id, _ =
        List.fold_result items ~init:(String.Map.empty, 0) ~f:(fun (by_id, bytes) item ->
          let key = Id.to_string item.id in
          let bytes = bytes + String.length key + String.length item.label in
          if bytes > max_text_bytes
          then Or_error.errorf "choice collection exceeds %d text bytes" max_text_bytes
          else (
            match Map.add by_id ~key ~data:item with
            | `Duplicate -> Or_error.errorf "duplicate choice id: %s" key
            | `Ok by_id -> Ok (by_id, bytes)))
      in
      { items; by_id })
  ;;

  let to_list t = t.items
  let find t id = Map.find t.by_id (Id.to_string id)

  let validate_selection t = function
    | None -> Ok ()
    | Some id ->
      if Map.mem t.by_id (Id.to_string id)
      then Ok ()
      else Or_error.errorf "selected choice does not exist: %s" (Id.to_string id)
  ;;
end

module Config = struct
  type t =
    { label : string
    ; options : Collection.t
    ; selected : Id.t option
    ; disabled : bool
    }
  [@@deriving equal, sexp_of]

  let create ~label ~options ~selected ?(disabled = false) () =
    let open Or_error.Let_syntax in
    let%bind () = validate_text ~name:"choice control label" ~max_bytes:1024 label in
    let%map () = Collection.validate_selection options selected in
    { label; options; selected; disabled }
  ;;

  let label t = t.label
  let options t = t.options
  let selected t = t.selected
  let is_disabled t = t.disabled

  let can_select t id =
    (not t.disabled)
    && Option.value_map (Collection.find t.options id) ~default:false ~f:(fun item ->
      not item.disabled)
  ;;
end

module Expert = struct
  let config_to_wire t : Gpuio_protocol.Wire.Choice.Config.t =
    { label = Config.label t
    ; items =
        List.map
          (Collection.to_list (Config.options t))
          ~f:(fun item ->
            ({ id = Id.to_string (id item)
             ; label = label item
             ; disabled = is_disabled item
             }
             : Gpuio_protocol.Wire.Choice.Item.t))
    ; selected = Option.map (Config.selected t) ~f:Id.to_string
    ; disabled = Config.is_disabled t
    }
  ;;
end
