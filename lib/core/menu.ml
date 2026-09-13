module Ui_command = Command
open Core

type t =
  { label : string
  ; disabled : bool
  ; items : item list
  }

and item =
  | Command of Ui_command.Id.t
  | Separator
  | Submenu of t
[@@deriving equal, sexp_of]

module Item = struct
  type nonrec t = item =
    | Command of Ui_command.Id.t
    | Separator
    | Submenu of t
  [@@deriving equal, sexp_of]
end

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
  let%bind.Or_error () =
    if
      String.is_empty (String.strip label)
      || String.length label > 4096
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
    then
      Or_error.error_string
        "menu label must be nonblank UTF-8 without NUL, at most 4096 bytes"
    else Ok ()
  in
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
  [@@deriving equal, sexp_of]

  let rec command_ids t =
    List.concat_map t.items ~f:(function
      | Command id -> [ id ]
      | Separator -> []
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
          | Submenu menu -> Submenu (to_wire menu))
    }
  ;;

  let validate_collection = validate_collection
end
