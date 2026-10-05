open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component window palette graph =
  let notice, set_notice = B.state "Focus a field, then press Primary+Shift+I." graph in
  let editor label ?(read_only = false) ?(privacy = Text_input.Privacy.Plain) initial_text
    =
    Editor.create
      window
      ~initial_text
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label ~read_only ~privacy () |> ok))
      graph
  in
  let draft = editor "Focus inspection draft" "A thought worth exploring" in
  let password =
    editor "Private focus inspection" ~privacy:(Password Hidden) "demo-secret"
  in
  let read_only =
    editor "Read-only focus inspection" ~read_only:true "Read-only is still a text input"
  in
  let open B.Let_syntax in
  let%arr p = palette
  and notice = notice
  and set_notice = set_notice
  and draft = draft
  and password = password
  and read_only = read_only in
  let inspect =
    E.bind (App.Window.focused_input window) ~f:(function
      | Ok None -> set_notice "No eligible native text input has keyboard focus."
      | Ok (Some input) ->
        let owner =
          List.find_map
            [ "Draft", draft; "Masked value", password; "Read-only value", read_only ]
            ~f:(fun (label, editor) ->
              if
                Option.exists
                  (Editor.snapshot editor)
                  ~f:(Window.Input.same_text_input input)
              then Some label
              else None)
          |> Option.value ~default:"Another native input"
        in
        set_notice
          (sprintf
             "Focused: %s · kind: %s · metadata only; no value read"
             owner
             (Sexp.to_string_hum [%sexp (Window.Input.kind input : Window.Input.Kind.t)]))
      | Error error -> set_notice (Sexp.to_string_hum [%sexp (error : Window.Error.t)]))
  in
  let shortcut =
    Shortcut.create ~key:"i" ~modifiers:[ Primary; Shift ] ~priority:Override () |> ok
  in
  let command =
    Command.create
      ~id:(Command.Id.of_string "gallery.inspect-focused-input" |> ok)
      ~label:"Inspect focused input"
      ~shortcuts:[ shortcut ]
      ~on_invoke:(fun () -> inspect)
      ()
    |> ok
  in
  let field label input =
    V.column
      ~style:(style [ Gap (px 6.) ])
      [ Palette.text p ~muted:true label
      ; Editor.view
          ~style:(style [ Height (px 36.); Width (Length.percent_exn 100.) ])
          input
      ]
  in
  V.command_scope
    ~commands:(Command.Registry.create [ command ] |> ok)
    [ Palette.card
        p
        ~title:"Which input owns focus?"
        [ Palette.text
            p
            ~muted:true
            "Use Command+Shift+I on macOS or Ctrl+Shift+I on Linux while editing. The \
             native query returns an input kind and mounted identity, never its text."
        ; field "Draft" draft
        ; field "Masked value" password
        ; field "Read-only value" read_only
        ; Palette.text p notice
        ]
    ]
;;
