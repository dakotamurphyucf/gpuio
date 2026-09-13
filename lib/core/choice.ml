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

module Appearance = struct
  type t =
    { popup_width : float
    ; row_height : float
    ; max_visible_rows : int
    ; empty_label : string
    ; popup_style : Style.t
    ; option_style : Style.t
    ; empty_style : Style.t
    }
  [@@deriving equal, sexp_of]

  let properties =
    let open Style.Property.Name in
    [ Background
    ; Foreground
    ; Opacity
    ; Border_color
    ; Shadows
    ; Top_left_radius
    ; Top_right_radius
    ; Bottom_left_radius
    ; Bottom_right_radius
    ; Font_size
    ; Font_family
    ; Font_weight
    ; Text_align
    ; Line_height
    ; White_space
    ; Text_overflow
    ; Line_clamp
    ; Text_decoration
    ; Cursor
    ]
  ;;

  let create
        ?(popup_width = 320.)
        ?(row_height = 32.)
        ?(max_visible_rows = 8)
        ?(empty_label = "No options")
        ?(popup_style = Style.empty)
        ?(option_style = Style.empty)
        ?(empty_style = Style.empty)
        ()
    =
    let valid value =
      Float.is_finite value && Float.(value > 0. && value <= 1_000_000.)
    in
    let open Or_error.Let_syntax in
    let%bind () =
      if
        valid popup_width
        && valid row_height
        && max_visible_rows >= 1
        && max_visible_rows <= 64
      then Ok ()
      else Or_error.error_string "invalid choice appearance geometry"
    in
    let%bind () = validate_text ~name:"empty choice label" ~max_bytes:1024 empty_label in
    let%bind () =
      Style.Expert.validate_scope popup_style ~states:[ Base; Hovered ] ~properties
    in
    let%bind () =
      Style.Expert.validate_scope
        option_style
        ~states:[ Base; Focused; Hovered; Pressed; Selected; Disabled ]
        ~properties
    in
    let%bind () = Style.Expert.validate_scope empty_style ~states:[ Base ] ~properties in
    let%map () =
      if
        List.sum
          (module Int)
          [ popup_style; option_style; empty_style ]
          ~f:Style.Expert.declaration_count
        <= 128
      then Ok ()
      else Or_error.error_string "choice appearance exceeds 128 declarations"
    in
    { popup_width
    ; row_height
    ; max_visible_rows
    ; empty_label
    ; popup_style
    ; option_style
    ; empty_style
    }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Expert = struct
  let appearance_to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let%bind popup_style = Style.Expert.to_wire t.popup_style ~theme in
    let%bind option_style = Style.Expert.to_wire t.option_style ~theme in
    let%map empty_style = Style.Expert.to_wire t.empty_style ~theme in
    ({ popup_width = t.popup_width
     ; row_height = t.row_height
     ; max_visible_rows = Int64.of_int t.max_visible_rows
     ; empty_label = t.empty_label
     ; popup_style
     ; option_style
     ; empty_style
     }
     : Gpuio_protocol.Wire.Choice_appearance.t)
  ;;

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
