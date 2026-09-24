open Core

let validate_text ~name ~max_bytes text =
  if
    String.is_empty (String.strip text)
    || String.length text > max_bytes
    || (not (Stdlib.String.is_valid_utf_8 text))
    || String.contains text '\000'
  then
    Or_error.errorf
      "%s must be nonblank UTF-8 without NUL and at most %d bytes"
      name
      max_bytes
  else Ok ()
;;

module Id = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string text =
    let%map.Or_error () = validate_text ~name:"command id" ~max_bytes:256 text in
    text
  ;;

  let to_string t = t
end

module Native = struct
  type t =
    | Copy
    | Cut
    | Paste
    | Select_all
    | Undo
    | Redo
  [@@deriving equal, sexp_of]
end

type 'action target =
  | Callback of (unit -> 'action)
  | Native of Native.t

type 'action t =
  { id : Id.t
  ; label : string
  ; enabled : bool
  ; checked : bool option
  ; shortcuts : Shortcut.t list
  ; target : 'action target
  }

let make ~id ~label ~enabled ~checked ~shortcuts ~target =
  let%bind.Or_error () = validate_text ~name:"command label" ~max_bytes:4096 label in
  if List.length shortcuts > 4
  then Or_error.error_string "a command has at most four shortcuts"
  else Ok { id; label; enabled; checked; shortcuts; target }
;;

let create ~id ~label ?(enabled = true) ?checked ?(shortcuts = []) ~on_invoke () =
  make ~id ~label ~enabled ~checked ~shortcuts ~target:(Callback on_invoke)
;;

let native ~id ~label ?(enabled = true) ?checked ?(shortcuts = []) action =
  make ~id ~label ~enabled ~checked ~shortcuts ~target:(Native action)
;;

let id t = t.id
let label t = t.label
let is_enabled t = t.enabled

module Registry = struct
  type 'action command = 'action t

  type 'action t =
    { commands : 'action command list
    ; by_id : 'action command String.Map.t
    }

  let create commands =
    if List.length commands > 1024
    then Or_error.error_string "command registry exceeds 1024 entries"
    else (
      let%map.Or_error by_id, _ =
        List.fold_result
          commands
          ~init:(String.Map.empty, 0)
          ~f:(fun (map, bytes) command ->
            let bytes =
              bytes
              + String.length command.label
              + String.length command.id
              + List.sum
                  (module Int)
                  command.shortcuts
                  ~f:(fun shortcut ->
                    String.length (Shortcut.Expert.to_wire shortcut).key)
            in
            if bytes > 262_144
            then Or_error.error_string "command registry exceeds 256 KiB of text"
            else (
              match Map.add map ~key:command.id ~data:command with
              | `Duplicate -> Or_error.errorf "duplicate command id: %s" command.id
              | `Ok map -> Ok (map, bytes)))
      in
      { commands; by_id })
  ;;

  let to_list t = t.commands
  let find t id = Map.find t.by_id id
end

module Expert = struct
  let to_wire t ~generation : Gpuio_protocol.Wire.Command.t =
    { id = t.id
    ; generation
    ; label = t.label
    ; enabled = t.enabled
    ; checked = t.checked
    ; shortcuts = List.map t.shortcuts ~f:Shortcut.Expert.to_wire
    ; target =
        (match t.target with
         | Callback _ -> Callback
         | Native native ->
           Native
             (match native with
              | Native.Copy -> Copy
              | Cut -> Cut
              | Paste -> Paste
              | Select_all -> Select_all
              | Undo -> Undo
              | Redo -> Redo))
    }
  ;;

  let invoke t =
    if not t.enabled
    then None
    else (
      match t.target with
      | Callback f -> Some (f ())
      | Native _ -> None)
  ;;
end
