open Core

module Modifier = struct
  type t =
    | Primary
    | Control
    | Alt
    | Shift
    | Super
  [@@deriving equal, compare, sexp_of]
end

module Priority = struct
  type t =
    | Native_first
    | Override
  [@@deriving equal, sexp_of]
end

module Text_input = struct
  type t =
    | Modified_only
    | Always
    | Never
  [@@deriving equal, sexp_of]
end

type t =
  { key : string
  ; modifiers : Modifier.t list
  ; priority : Priority.t
  ; text_input : Text_input.t
  ; during_composition : bool
  }
[@@deriving equal, sexp_of]

let valid_key key =
  let named =
    List.mem
      [ "enter"
      ; "escape"
      ; "tab"
      ; "space"
      ; "backspace"
      ; "delete"
      ; "insert"
      ; "home"
      ; "end"
      ; "pageup"
      ; "pagedown"
      ; "left"
      ; "right"
      ; "up"
      ; "down"
      ]
      key
      ~equal:String.equal
  in
  let function_key =
    List.exists
      (List.init 24 ~f:(fun index -> "f" ^ Int.to_string (index + 1)))
      ~f:(String.equal key)
  in
  let scalar =
    Stdlib.String.is_valid_utf_8 key
    && String.length key > 0
    && String.count key ~f:(fun byte -> Char.to_int byte land 0xc0 <> 0x80) = 1
    && (not
          (String.length key = 2
           && Char.to_int key.[0] = 0xc2
           && Char.to_int key.[1] <= 0x9f))
    && String.for_all key ~f:(fun byte ->
      Char.to_int byte >= 0x21 && Char.to_int byte <> 0x7f)
  in
  named || function_key || scalar
;;

let create
      ~key
      ?(modifiers = [])
      ?(priority = Priority.Native_first)
      ?(text_input = Text_input.Modified_only)
      ?(during_composition = false)
      ()
  =
  let key = String.lowercase key in
  let sorted = List.sort modifiers ~compare:Modifier.compare in
  if not (valid_key key)
  then Or_error.error_string "invalid shortcut key"
  else if List.contains_dup sorted ~compare:Modifier.compare
  then Or_error.error_string "duplicate shortcut modifier"
  else Ok { key; modifiers = sorted; priority; text_input; during_composition }
;;

module Expert = struct
  let to_wire t : Gpuio_protocol.Wire.Shortcut.t =
    { key = t.key
    ; modifiers =
        List.map t.modifiers ~f:(function
          | Modifier.Primary -> Gpuio_protocol.Wire.Shortcut_modifier.Primary
          | Control -> Control
          | Alt -> Alt
          | Shift -> Shift
          | Super -> Super)
    ; priority =
        (match t.priority with
         | Native_first -> Native_first
         | Override -> Override)
    ; text_input =
        (match t.text_input with
         | Modified_only -> Modified_only
         | Always -> Always
         | Never -> Never)
    ; during_composition = t.during_composition
    }
  ;;
end
