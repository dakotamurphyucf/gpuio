module Ui_command = Command
open Core

module Item_path = struct
  type t = int list [@@deriving equal, sexp_of]

  let of_list indices =
    match indices with
    | menu :: (_ :: _ as items)
      when menu >= 0
           && menu < 32
           && List.length items <= 8
           && List.for_all items ~f:(fun index -> index >= 0 && index < 1024) ->
      Ok indices
    | _ -> Or_error.error_string "menu item path exceeds menu, item or depth bounds"
  ;;

  let to_list t = t
end

type t =
  { label : string
  ; disabled : bool
  ; items : item list
  }

and item =
  | Command of Ui_command.Id.t
  | Separator
  | Submenu of t
  | Label of string
[@@deriving equal, sexp_of]

module Item = struct
  type nonrec t = item =
    | Command of Ui_command.Id.t
    | Separator
    | Submenu of t
    | Label of string
  [@@deriving equal, sexp_of]
end

let validate_label label =
  if
    String.is_empty (String.strip label)
    || String.length label > 4096
    || (not (Stdlib.String.is_valid_utf_8 label))
    || String.contains label '\000'
  then
    Or_error.error_string
      "menu label must be nonblank UTF-8 without NUL, at most 4096 bytes"
  else Ok ()
;;

let rec measure t ~depth ~count ~bytes =
  let count = count + List.length t.items in
  let bytes = bytes + String.length t.label in
  if depth > 8 || count > 1024 || bytes > 262_144
  then Or_error.error_string "menu exceeds depth, item or text limit"
  else
    List.fold_result t.items ~init:(count, bytes) ~f:(fun (count, bytes) -> function
      | Command id ->
        let bytes = bytes + String.length (Ui_command.Id.to_string id) in
        if bytes > 262_144
        then Or_error.error_string "menu exceeds 256 KiB of text"
        else Ok (count, bytes)
      | Label label ->
        let%bind.Or_error () = validate_label label in
        let bytes = bytes + String.length label in
        if bytes > 262_144
        then Or_error.error_string "menu exceeds 256 KiB of text"
        else Ok (count, bytes)
      | Separator -> Ok (count, bytes)
      | Submenu menu -> measure menu ~depth:(depth + 1) ~count ~bytes)
;;

let validate_collection menus =
  if List.length menus > 32
  then Or_error.error_string "menu bar exceeds 32 menus"
  else
    List.fold_result menus ~init:(0, 0) ~f:(fun (count, bytes) menu ->
      measure menu ~depth:1 ~count ~bytes)
    |> Or_error.map ~f:(fun _ -> ())
;;

let create ~label ?(disabled = false) items =
  let%bind.Or_error () = validate_label label in
  let t = { label; disabled; items } in
  let%map.Or_error () = validate_collection [ t ] in
  t
;;

let label t = t.label

module Appearance = Choice.Appearance

module Expert = struct
  type presentation =
    | Button
    | Context
    | Bar
    | Platform_bar
    | Editor_context
    | Platform_context
  [@@deriving equal, sexp_of]

  let item_paths menus =
    let rec visit reversed_prefix menu =
      List.concat_mapi menu.items ~f:(fun index item ->
        let path = index :: reversed_prefix in
        let descendants =
          match item with
          | Submenu child -> visit path child
          | Command _ | Separator | Label _ -> []
        in
        (List.rev path, item) :: descendants)
    in
    List.concat_mapi menus ~f:(fun index menu -> visit [ index ] menu)
  ;;

  let rec command_ids t =
    List.concat_map t.items ~f:(function
      | Command id -> [ id ]
      | Separator | Label _ -> []
      | Submenu menu -> command_ids menu)
  ;;

  let rec to_wire t : Gpuio_protocol.Wire.Menu_definition.t =
    { label = t.label
    ; disabled = t.disabled
    ; items =
        List.map t.items ~f:(function
          | Command id ->
            Gpuio_protocol.Wire.Menu_definition.Command (Ui_command.Id.to_string id)
          | Separator -> Separator
          | Label label -> Label label
          | Submenu menu -> Submenu (to_wire menu))
    }
  ;;

  let validate_collection = validate_collection

  let validate_platform_collection menus =
    let rec contains_labels menu =
      List.exists menu.items ~f:(function
        | Label _ -> true
        | Command _ | Separator -> false
        | Submenu child -> contains_labels child)
    in
    let%bind.Or_error () = validate_collection menus in
    if List.exists menus ~f:contains_labels
    then
      Or_error.error_string
        "platform menu bars do not support section labels; use platform:false"
    else Ok ()
  ;;
end
