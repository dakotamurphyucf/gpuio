open Core
open Gpuio
module App = Gpuio_eio.App
module D = Gpuio_eio.Desktop
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

let result_message label sexp = function
  | Ok _ -> label ^ ": request accepted"
  | Error e -> label ^ ": " ^ Sexp.to_string_hum (sexp e)
;;

let component app session window palette graph =
  let document, set_document = B.state None graph in
  let notice, set_notice = B.state "No file action requested" graph in
  let open B.Let_syntax in
  let refresh =
    let%arr set_document = set_document in
    E.bind (App.Window.command window Observe) ~f:(function
      | Ok snapshot -> set_document (Window.Document.of_snapshot snapshot)
      | Error _ -> E.Ignore)
  in
  B.Edge.lifecycle ~on_activate:refresh graph;
  let%arr p = palette
  and state = Desktop_session.snapshot session
  and document = document
  and set_document = set_document
  and notice = notice
  and set_notice = set_notice in
  let row children = V.row ~style:(style [ Gap (px 8.); Wrap Wrap ]) children in
  let current =
    Option.value document ~default:(Window.Document.create ~edited:false ())
  in
  let path = Window.Document.path current in
  let apply document =
    E.bind (D.set_document window document) ~f:(function
      | Ok snapshot ->
        E.Many
          [ set_document (Window.Document.of_snapshot snapshot)
          ; set_notice "Document metadata observed"
          ]
      | Error e ->
        set_notice
          ("Document metadata: " ^ Sexp.to_string_hum [%sexp (e : Window.Error.t)]))
  in
  let choose =
    E.bind
      (Gpuio_eio.File_dialog.open_
         window
         ~config:
           (File_dialog.Open.create
              ~title:"Represent a gallery file"
              ~accept_label:"Represent"
              ()
            |> ok))
      ~f:(function
        | Ok None -> set_notice "Document selection cancelled"
        | Ok (Some [ path ]) -> apply (Window.Document.create ~path ~edited:false ())
        | Ok (Some _) -> set_notice "Choose one represented file"
        | Error e ->
          set_notice
            ("Document selection: " ^ Sexp.to_string_hum [%sexp (e : File_dialog.Error.t)]))
  in
  let file_action label operation =
    match path with
    | None -> E.Ignore
    | Some path ->
      E.bind (operation app path) ~f:(fun result ->
        set_notice (result_message label D.Error.sexp_of_t result))
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"One identity, every window"
        [ Palette.text p (D.Identity.name Desktop_session.identity)
        ; Palette.text p ~muted:true (D.Identity.identifier Desktop_session.identity)
        ; row
            [ Palette.button
                p
                ~disabled:state.busy
                "Check desktop support"
                (Desktop_session.check session)
            ; Palette.button p "Retry incoming events" (Desktop_session.retry session)
            ; Palette.button
                p
                "Activate application"
                (E.bind (D.activate app ()) ~f:(fun result ->
                   set_notice
                     (result_message "Application activation" D.Error.sexp_of_t result)))
            ]
        ; Palette.text p state.desktop_support
        ; Palette.text p state.link
        ; Palette.text
            p
            ~muted:true
            "Incoming links are observed without opening files or changing windows."
        ; Palette.button
            p
            "Register Studio links with the OS"
            (E.bind (D.register_scheme app Desktop_session.scheme) ~f:(fun result ->
               set_notice (result_message "Link registration" D.Error.sexp_of_t result)))
        ]
    ; Palette.card
        p
        ~title:"A document belongs to its window"
        [ row
            [ Palette.button p "Choose represented file" choose
            ; Palette.button
                p
                "Clear represented file"
                (apply (Window.Document.create ~edited:false ()))
            ]
        ; row
            [ Palette.button
                p
                "Mark document edited"
                (apply (Window.Document.create ?path ~edited:true ()))
            ; Palette.button
                p
                "Mark document saved"
                (apply (Window.Document.create ?path ~edited:false ()))
            ; Palette.button
                p
                ~disabled:(Option.is_none path)
                "Reveal represented file"
                (file_action "File reveal" D.reveal_file)
            ; Palette.button
                p
                ~disabled:(Option.is_none path)
                "Open represented file"
                (file_action "File open" D.open_file)
            ]
        ; Palette.text
            p
            (if Option.is_some path
             then "Represented file: selected"
             else "Represented file: none")
        ; Palette.text
            p
            ("Document edited: " ^ Bool.to_string (Window.Document.edited current))
        ; Palette.text p notice
        ; Palette.text
            p
            ~muted:true
            "Selecting a file only changes window metadata. Open and Reveal are separate \
             OS actions; marking saved does not write a file."
        ]
    ; Palette.card
        p
        ~title:"A small update, beyond the window"
        [ Palette.text p state.notification_support
        ; Palette.text p state.authorization
        ; row
            [ Palette.button
                p
                ~disabled:state.busy
                "Allow OS notifications"
                (Desktop_session.allow session)
            ; Palette.button
                p
                ~disabled:(state.busy || state.has_receipt)
                "Post preview notification"
                (Desktop_session.post session)
            ; Palette.button
                p
                ~disabled:(state.busy || not state.has_receipt)
                "Replace preview notification"
                (Desktop_session.replace session)
            ; Palette.button
                p
                ~disabled:(state.busy || not state.has_receipt)
                "Dismiss preview notification"
                (Desktop_session.dismiss session)
            ]
        ; Palette.text p state.notice
        ; Palette.text
            p
            ~muted:true
            "Checking support never prompts. On macOS, notifications and link \
             registration need the packaged application. One notification is shared \
             across all Studio windows."
        ]
    ]
;;
