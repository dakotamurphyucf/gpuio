module Ui_command = Command
open Core
module Wire = Gpuio_protocol.Wire

module Config = struct
  type t =
    { label : string
    ; placeholder : string
    ; commands : Ui_command.Id.t list
    ; dismiss_on_outside_pointer : bool
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ~commands
        ?(placeholder = "Search commands")
        ?(dismiss_on_outside_pointer = true)
        ()
    =
    let text value =
      String.length value <= 4096
      && Stdlib.String.is_valid_utf_8 value
      && not (String.contains value '\000')
    in
    let ids = List.map commands ~f:Ui_command.Id.to_string in
    if (not (text label && text placeholder)) || String.is_empty (String.strip label)
    then
      Or_error.error_string
        "palette label and placeholder must be bounded UTF-8 without NUL; label must be \
         nonblank"
    else if
      List.length ids > 1024 || Set.length (String.Set.of_list ids) <> List.length ids
    then Or_error.error_string "palette requires at most 1024 unique command IDs"
    else if
      String.length label
      + String.length placeholder
      + List.sum (module Int) ids ~f:String.length
      > 262_144
    then Or_error.error_string "palette metadata exceeds 256 KiB"
    else Ok { label; placeholder; commands; dismiss_on_outside_pointer }
  ;;

  let commands t = t.commands
end

module Dismissal = struct
  type t =
    | Escape
    | Outside_pointer
    | Selected of Ui_command.Id.t
  [@@deriving equal, sexp_of]
end

module Appearance = Choice.Appearance

module Expert = struct
  let to_wire (t : Config.t) : Wire.Palette.t =
    { label = t.label
    ; placeholder = t.placeholder
    ; commands = List.map t.commands ~f:Ui_command.Id.to_string
    ; dismiss_on_outside_pointer = t.dismiss_on_outside_pointer
    }
  ;;

  let dismissal t : Wire.Palette_dismissal.t -> Dismissal.t option = function
    | Escape -> Some Escape
    | Outside_pointer ->
      if t.Config.dismiss_on_outside_pointer then Some Outside_pointer else None
    | Selected id ->
      List.find t.commands ~f:(fun command ->
        String.equal (Ui_command.Id.to_string command) id)
      |> Option.map ~f:(fun id -> Dismissal.Selected id)
  ;;
end
