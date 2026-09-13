open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Command = Gpuio.Command
module Palette = Gpuio.Command_palette

let id value = Command.Id.of_string value |> Or_error.ok_exn
let run = id "run"
let copy = id "copy"
let open_palette = id "open-palette"

let config =
  Palette.Config.create ~label:"Commands" ~commands:[ run; copy ] () |> Or_error.ok_exn
;;

let component ~phase ~observed window graph =
  let count, set_count = B.state 0 graph in
  let open_, set_open = B.state false graph in
  let input =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create
              ~mode:Single_line
              ~label:"Draft"
              ~auto_focus:true
              ()
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
  and open_ = open_
  and set_open = set_open
  and input = input
  and phase = phase in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:run
          ~label:(sprintf "Run (%d)" count)
          ~enabled:(phase <> 2)
          ~on_invoke:(fun () -> set_count (count + 1))
          ()
        |> Or_error.ok_exn
      ; Command.native ~id:copy ~label:"Copy draft selection" Copy |> Or_error.ok_exn
      ; Command.create
          ~id:open_palette
          ~label:"Open commands"
          ~shortcuts:
            [ Gpuio.Shortcut.create ~key:"p" ~modifiers:[ Primary; Shift ] ()
              |> Or_error.ok_exn
            ]
          ~on_invoke:(fun () -> set_open true)
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
    ([ View.text
         "Write a draft and select text. Use the command chooser to run an action or \
          copy that selection."
     ; Gpuio_eio.Text_input.view input
     ; View.command_button ~command:open_palette ()
     ; View.text (sprintf "Runs: %d" count)
     ]
     @
     if open_ || phase = 1 || phase = 2
     then [ View.command_palette ~config ~on_dismiss:(fun _ -> set_open false) () ]
     else [])
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
        ~title:"GPUIO commands"
        ~width:660.
        ~height:480.
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
            let opened = rendered 1 in
            let disabled = rendered 2 in
            let closed = rendered 3 in
            assert (Int64.(initial < opened && opened < disabled && disabled < closed))))
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
      "GPUIO_PALETTE_PUBLIC_OK: public native palette mount, command metadata update, \
       unmount and shutdown")
;;
