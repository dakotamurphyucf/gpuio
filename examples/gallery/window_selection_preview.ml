open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module App = Gpuio_eio.App

let component window palette graph =
  let notice, set_notice = B.state "Select some of the lines below." graph in
  let open B.Let_syntax in
  let%arr p = palette
  and notice = notice
  and set_notice = set_notice in
  let error error = set_notice (Sexp.to_string_hum [%sexp (error : Window.Error.t)]) in
  let inspect =
    E.bind (App.Window.selected_text window ()) ~f:(function
      | Ok "" -> set_notice "No text is selected."
      | Ok text ->
        set_notice (sprintf "Selected %d UTF-8 bytes: %S" (String.length text) text)
      | Error reason -> error reason)
  in
  let check =
    E.bind (App.Window.has_text_selection window) ~f:(function
      | Ok present ->
        set_notice
          (if present then "Selection is present." else "No selection is present.")
      | Error reason -> error reason)
  in
  let action operation message =
    E.bind operation ~f:(function
      | Ok () -> set_notice message
      | Error reason -> error reason)
  in
  let command key label operation =
    Command.create
      ~id:(Command.Id.of_string ("gallery.selection." ^ key) |> Or_error.ok_exn)
      ~label
      ~shortcuts:
        [ Shortcut.create ~key ~modifiers:[ Primary; Shift ] ~priority:Override ()
          |> Or_error.ok_exn
        ]
      ~on_invoke:(fun () -> operation)
      ()
    |> Or_error.ok_exn
  in
  let clear = action (App.Window.clear_text_selection window) "Selection cleared." in
  let end_drag =
    action (App.Window.end_text_selection window) "Drag ended; selection preserved."
  in
  V.command_scope
    ~commands:
      (Command.Registry.create
         [ command "u" "Inspect selected text" inspect
         ; command "e" "End selection drag" end_drag
         ; command "y" "Check selection presence" check
         ; command "k" "Clear text selection" clear
         ]
       |> Or_error.ok_exn)
    [ Palette.card
        p
        ~title:"Selection without the clipboard"
        [ Palette.text
            p
            ~muted:true
            "Drag across the lines, then press Primary+Shift+U to inspect. \
             Primary+Shift+E ends a drag and keeps the range. Use Primary+Shift+Y to \
             check selection or Primary+Shift+K to clear it. Clipboard contents stay \
             untouched."
        ; V.column
            ~style:(Style.create_exn [ User_select true; Gap (Length.px_exn 8.) ])
            [ Palette.text p "A quiet workspace for your next idea."
            ; Palette.text p "Unicode stays whole: café · 京都 · 👩‍💻"
            ; Palette.text p "Selections can span several text nodes."
            ]
        ; V.row
            ~style:(Style.create_exn [ Gap (Length.px_exn 8.); Wrap Wrap ])
            [ Palette.button p "Check selection" check
            ; Palette.button p "Clear selection" clear
            ]
        ; Palette.text p notice
        ]
    ]
;;
