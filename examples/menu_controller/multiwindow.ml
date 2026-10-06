open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Menu = Gpuio.Menu
module Command = Gpuio.Command
module Controller = Gpuio_eio.Menu_controller
module Input = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let run_id = Command.Id.of_string "run" |> ok
let copy_id = Command.Id.of_string "copy" |> ok
let position = Menu.Position.create ~x:60. ~y:120. |> ok

let create ~name ~platform window graph =
  let controller = Controller.create window graph in
  let count, set_count = B.state 0 graph in
  let status, set_status = B.state "Ready" graph in
  let first_visible, toggle_first = B.toggle ~default_model:true graph in
  let editor label =
    Input.create
      window
      ~initial_text:(name ^ " " ^ label)
      ~config:
        (B.return (Gpuio.Text_input.Config.create ~mode:Single_line ~label () |> ok))
      graph
  in
  let first = editor "First editor" in
  let second = editor "Second editor" in
  let open B.Let_syntax in
  let%arr controller = controller
  and count = count
  and set_count = set_count
  and status = status
  and set_status = set_status
  and first_visible = first_visible
  and toggle_first = toggle_first
  and first = first
  and second = second in
  let menu_command label command =
    E.map (Controller.command controller command) ~f:(function
      | Ok () -> "accepted"
      | Error error -> Sexp.to_string (Menu.Command_error.sexp_of_t error))
    |> fun result -> E.bind result ~f:(fun result -> set_status (label ^ ": " ^ result))
  in
  let focus label input =
    let open E.Let_syntax in
    let%bind result = Input.focus input in
    set_status
      (label
       ^ ": "
       ^
       match result with
       | Ok _ -> "applied"
       | Error error -> Sexp.to_string (Gpuio.Text_input.Command_error.sexp_of_t error))
  in
  let window_command label command =
    let open E.Let_syntax in
    let%bind result = Gpuio_eio.App.Window.command window command in
    set_status
      (label
       ^ ": "
       ^
       match result with
       | Ok snapshot -> "active=" ^ Bool.to_string snapshot.active
       | Error error -> Sexp.to_string (Gpuio.Window.Error.sexp_of_t error))
  in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:run_id
          ~label:"Run"
          ~on_invoke:(fun () -> set_count (count + 1))
          ()
        |> ok
      ; Command.native ~id:copy_id ~label:"Copy selection" Copy |> ok
      ]
    |> ok
  in
  let menu =
    Menu.create ~label:(name ^ " actions") [ Command run_id; Command copy_id ] |> ok
  in
  let ready = Option.is_some (Controller.snapshot controller) in
  V.command_scope
    ~commands
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 20.); Gap (Gpuio.Length.px_exn 12.) ])
    [ V.text (name ^ " — independent menus and editors")
    ; V.text (sprintf "Total runs: %d" count)
    ; V.text status
    ; V.button ~on_click:(window_command "Activate" Activate) "Activate window"
    ; V.button ~on_click:(window_command "Observe" Observe) "Observe window"
    ; V.context_menu
        ~platform
        ~key:(Controller.key controller)
        ~on_change:(Controller.observe controller)
        ~menu
        (V.column
           [ (if first_visible then Input.view first else V.text "First editor removed")
           ; Input.view second
           ])
    ; V.button
        ~disabled:(not ready)
        ~on_click:(menu_command "Open popup" (Show position))
        "Open popup"
    ; V.button
        ~disabled:(not ready)
        ~on_click:(menu_command "Close popup" Close)
        "Close popup"
    ; V.button
        ~disabled:(not first_visible)
        ~on_click:(focus "First focus" first)
        "Focus first editor"
    ; V.button ~on_click:(focus "Second focus" second) "Focus second editor"
    ; V.button
        ~on_click:toggle_first
        (if first_visible then "Remove first editor" else "Restore first editor")
    ; V.button
        ~on_click:(E.of_thunk (fun () -> Gpuio_eio.App.Window.close window))
        "Close window"
    ]
;;
