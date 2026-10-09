module Ui_command = Command
open Core

type t =
  { enabled : bool
  ; label : string
  ; cut : string
  ; copy : string
  ; paste : string
  ; select_all : string
  }
[@@deriving equal, sexp_of]

let create
      ?(enabled = true)
      ?(label = "Edit")
      ?(cut = "Cut")
      ?(copy = "Copy")
      ?(paste = "Paste")
      ?(select_all = "Select all")
      ()
  =
  let labels = [ label; cut; copy; paste; select_all ] in
  if
    List.exists labels ~f:(fun label ->
      String.is_empty (String.strip label)
      || String.length label > 4096
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000')
  then
    Or_error.error_string
      "editor menu labels must be nonblank UTF-8 without NUL, at most 4096 bytes"
  else Ok { enabled; label; cut; copy; paste; select_all }
;;

let default = create () |> Or_error.ok_exn

module Expert = struct
  let id name = Ui_command.Id.of_string ("gpuio.editor-menu." ^ name) |> Or_error.ok_exn

  let menu t =
    Menu.create
      ~label:t.label
      ~disabled:(not t.enabled)
      [ Command (id "cut")
      ; Command (id "copy")
      ; Command (id "paste")
      ; Separator
      ; Command (id "select-all")
      ]
    |> Or_error.ok_exn
  ;;

  let commands t =
    List.map
      [ "cut", t.cut, Ui_command.Native.Cut
      ; "copy", t.copy, Copy
      ; "paste", t.paste, Paste
      ; "select-all", t.select_all, Select_all
      ]
      ~f:(fun (name, label, action) ->
        Ui_command.native ~id:(id name) ~label action |> Or_error.ok_exn)
    |> Ui_command.Registry.create
    |> Or_error.ok_exn
  ;;
end
