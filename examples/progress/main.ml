open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module Ui_view = Gpuio_bonsai.View
module App = Gpuio_eio.App
module Progress = Gpuio.Progress

let component ~phase ~observed _window graph =
  let step, set_step = B.state 0 graph in
  let phase = B.Expert.Var.value phase in
  B.Edge.on_change
    phase
    ~equal:Int.equal
    ~callback:(B.return (fun phase -> E.of_thunk (fun () -> observed := phase)))
    graph;
  let open B.Let_syntax in
  let%arr step = step
  and set_step = set_step
  and phase = phase in
  let current = if phase < 0 then step else phase in
  let value =
    match current with
    | 1 -> Progress.Value.indeterminate
    | _ ->
      Progress.Value.determinate
        ~fraction:(Float.of_int (Int.min 2 (Int.max 0 (current - 1))) /. 2.)
      |> Or_error.ok_exn
  in
  let config =
    Progress.Config.create ~label:"Download progress" ~value |> Or_error.ok_exn
  in
  let open Gpuio in
  let style =
    Style.create_exn
      [ Width (Length.px_exn 360.)
      ; Height (Length.px_exn 12.)
      ; Foreground (Color.rgb_exn 0x2288cc)
      ]
  in
  Ui_view.column
    ~style:(Style.create_exn [ Padding (Length.px_exn 24.); Gap (Length.px_exn 16.) ])
    ([ Ui_view.text "Native progress: empty, indeterminate, halfway, complete"
     ; Ui_view.button ~on_click:(set_step ((step + 1) % 4)) "Next stage"
     ]
     @ if current = 4 then [] else [ Ui_view.progress ~style ~config () ])
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let phase = B.Expert.Var.create (-1) in
    let observed = ref (-2) in
    let window =
      App.open_window
        app
        ~title:"GPUIO progress"
        ~width:600.
        ~height:260.
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
            let rendered value =
              B.Expert.Var.set phase value;
              while !observed <> value do
                Eio.Time.sleep clock 0.005
              done;
              let promise, resolver = Eio.Promise.create () in
              App.Window.request_frame window ~on_rendered:(fun ~revision ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver revision))
              |> Or_error.ok_exn;
              Eio.Promise.await promise
            in
            let initial = rendered 0 in
            let busy = rendered 1 in
            let halfway = rendered 2 in
            let done_ = rendered 3 in
            let removed = rendered 4 in
            assert (
              Int64.(
                initial < busy && busy < halfway && halfway < done_ && done_ < removed))))
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
      "GPUIO_PROGRESS_PUBLIC_OK: native mount, indeterminate/determinate updates, \
       unmount and shutdown")
;;
