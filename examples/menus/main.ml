open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Command = Gpuio.Command
module Menu = Gpuio.Menu

let command_id name = Command.Id.of_string name |> Or_error.ok_exn
let run_id = command_id "run"
let copy_id = command_id "copy"
let quit_id = command_id "quit"
let shortcut key = Gpuio.Shortcut.create ~key ~modifiers:[ Primary ] () |> Or_error.ok_exn
let file_menu = Menu.create ~label:"File" [ Command quit_id ] |> Or_error.ok_exn

let actions_menu =
  let more = Menu.create ~label:"More actions" [ Command run_id ] |> Or_error.ok_exn in
  Menu.create
    ~label:"Actions"
    [ Command run_id; Command copy_id; Separator; Submenu more ]
  |> Or_error.ok_exn
;;

let component ~phase ~observed window graph =
  let count, set_count = B.state 0 graph in
  let enabled, set_enabled = B.state true graph in
  let input =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create ~mode:Single_line ~label:"Draft" ()
            |> Or_error.ok_exn))
      graph
  in
  let phase = B.Expert.Var.value phase in
  B.Edge.on_change
    phase
    ~equal:Int.equal
    ~callback:(B.return (fun phase -> E.of_thunk (fun () -> observed := phase)))
    graph;
  let open B.Let_syntax in
  let%arr count = count
  and set_count = set_count
  and enabled = enabled
  and set_enabled = set_enabled
  and input = input
  and phase = phase in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:run_id
          ~label:(sprintf "Run (%d)" count)
          ~enabled:(enabled && phase <> 1)
          ~shortcuts:[ shortcut "k" ]
          ~on_invoke:(fun () -> set_count (count + 1))
          ()
        |> Or_error.ok_exn
      ; Command.native ~id:copy_id ~label:"Copy selection" Copy |> Or_error.ok_exn
      ; Command.create
          ~id:quit_id
          ~label:"Close window"
          ~on_invoke:(fun () -> E.of_thunk (fun () -> App.Window.close window))
          ()
        |> Or_error.ok_exn
      ]
    |> Or_error.ok_exn
  in
  let inner_commands =
    Command.Registry.create
      [ Command.create
          ~id:run_id
          ~label:"Run ten"
          ~shortcuts:[ shortcut "k" ]
          ~on_invoke:(fun () -> set_count (count + 10))
          ()
        |> Or_error.ok_exn
      ]
    |> Or_error.ok_exn
  in
  let style =
    Gpuio.Style.create_exn
      [ Padding (Gpuio.Length.px_exn 20.); Gap (Gpuio.Length.px_exn 12.) ]
  in
  View.command_scope
    ~commands
    ~style
    [ View.menu_bar [ file_menu; actions_menu ] |> Or_error.ok_exn
    ; View.text
        "Menus and shortcuts share the same commands. Right-click the draft for its \
         context menu."
    ; View.menu_button ~menu:actions_menu ()
    ; View.text (sprintf "Total runs: %d" count)
    ; View.context_menu ~menu:actions_menu (Gpuio_eio.Text_input.view input)
    ; View.row
        [ View.command_button ~command:run_id ()
        ; View.command_button ~command:copy_id ()
        ]
    ; View.checkbox
        ~state:(if enabled then Checked else Unchecked)
        ~on_toggle:(set_enabled (not enabled))
        "Enable the outer Run command"
    ; (if phase = 3
       then View.text "Inner scope removed"
       else
         View.command_scope
           ~key:(Gpuio.Key.of_string_exn "inner")
           ~commands:inner_commands
           [ View.text "Focus this button to use the inner command."
           ; View.menu_button ~menu:actions_menu ()
           ])
    ; View.command_button ~command:quit_id ()
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let phase = B.Expert.Var.create 0 in
    let observed = ref (-1) in
    let window =
      App.open_window
        app
        ~title:"GPUIO menus"
        ~width:660.
        ~height:520.
        (component ~phase ~observed)
      |> Or_error.ok_exn
    in
    if self_test
    then (
      let clock = Eio.Stdenv.clock env in
      Gpuio_eio.Scope.start
        (App.scope app)
        ~f:(fun () ->
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            let rendered phase_value =
              B.Expert.Var.set phase phase_value;
              while !observed <> phase_value do
                Eio.Time.sleep clock 0.005
              done;
              let promise, resolver = Eio.Promise.create () in
              App.Window.request_frame window ~on_rendered:(fun ~revision ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver revision))
              |> Or_error.ok_exn;
              Eio.Promise.await promise
            in
            let initial = rendered 0 in
            let disabled = rendered 1 in
            let enabled = rendered 2 in
            let removed = rendered 3 in
            assert (Int64.(initial < disabled && disabled < enabled && enabled < removed))))
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            Or_error.ok_exn result;
            completed := true;
            App.Window.close window))
      |> Or_error.ok_exn
      |> fun (_ : Gpuio_eio.Scope.Task.t) -> ()));
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_MENUS_PUBLIC_OK: public menu presentations, nested registries, native \
       render acknowledgements, disable/re-enable, scope removal and clean shutdown")
;;
