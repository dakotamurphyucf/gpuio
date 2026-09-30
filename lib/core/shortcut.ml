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

module Platform = struct
  type t =
    | Macos
    | Linux
  [@@deriving equal, sexp_of]
end

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

let uppercase_scalar text =
  let scalar = Stdlib.Uchar.utf_decode_uchar (Stdlib.String.get_utf_8_uchar text 0) in
  match Uucp.Case.Map.to_upper scalar with
  | `Self -> text
  | `Uchars scalars ->
    let buffer = Stdlib.Buffer.create 8 in
    List.iter scalars ~f:(Stdlib.Buffer.add_utf_8_uchar buffer);
    Stdlib.Buffer.contents buffer
;;

let key_label key ~macos ~spoken =
  let symbol glyph name = if macos && not spoken then glyph else name in
  match key with
  | "enter" -> symbol "⏎" "Enter"
  | "escape" -> symbol "⎋" (if spoken then "Escape" else "Esc")
  | "tab" -> "Tab"
  | "space" -> "Space"
  | "backspace" -> symbol "⌫" "Backspace"
  | "delete" -> symbol "⌦" "Delete"
  | "insert" -> "Insert"
  | "home" -> "Home"
  | "end" -> "End"
  | "pageup" -> "Page Up"
  | "pagedown" -> "Page Down"
  | "left" -> symbol "←" (if spoken then "Left arrow" else "Left")
  | "right" -> symbol "→" (if spoken then "Right arrow" else "Right")
  | "up" -> symbol "↑" (if spoken then "Up arrow" else "Up")
  | "down" -> symbol "↓" (if spoken then "Down arrow" else "Down")
  | _ ->
    if String.length key > 1 && Char.equal key.[0] 'f'
    then String.capitalize key
    else uppercase_scalar key
;;

let label t ~platform ~spoken =
  let macos = Platform.equal platform Macos in
  let has modifier = List.mem t.modifiers modifier ~equal:Modifier.equal in
  let control = has Control || ((not macos) && has Primary) in
  let super = has Super || (macos && has Primary) in
  let symbol glyph name = if macos && not spoken then glyph else name in
  let modifiers =
    List.filter_opt
      [ Option.some_if control (symbol "⌃" (if spoken then "Control" else "Ctrl"))
      ; Option.some_if
          (has Alt)
          (symbol "⌥" (if macos && spoken then "Option" else "Alt"))
      ; Option.some_if (has Shift) (symbol "⇧" "Shift")
      ; Option.some_if super (symbol "⌘" (if macos then "Command" else "Super"))
      ]
  in
  String.concat
    ~sep:(if spoken then " + " else if macos then "" else "+")
    (modifiers @ [ key_label t.key ~macos ~spoken ])
;;

let format t ~platform = label t ~platform ~spoken:false
let accessible_label t ~platform = label t ~platform ~spoken:true

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
