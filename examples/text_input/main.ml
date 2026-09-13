open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Input = Gpuio.Text_input
module Controller = Gpuio_eio.Text_input
module View = Gpuio_bonsai.View

let component ~self_test ~completed ~mode window graph =
  let open B.Let_syntax in
  let submission, set_submission = B.state "Press Enter to submit" graph in
  let shown, set_shown = B.state true graph in
  let sleep = B.Clock.sleep graph in
  let started = ref false in
  let config =
    Input.Config.create
      ~mode
      ~label:"Message"
      ~placeholder:"Type a message"
      ~auto_focus:true
      ()
    |> Or_error.ok_exn
  in
  let on_submit =
    let%arr set_submission = set_submission in
    fun value -> set_submission ("Submitted: " ^ Input.Submission.text value)
  in
  let editor = Controller.create window ~config:(B.return config) ~on_submit graph in
  let observed = B.map editor ~f:Controller.snapshot in
  B.Edge.on_change
    observed
    ~equal:(Option.equal Input.Snapshot.equal)
    ~callback:
      (let%arr editor = editor
       and sleep = sleep
       and set_shown = set_shown in
       fun observation ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             if self_test && Option.is_some observation && not !started
             then (
               started := true;
               true)
             else false)
         in
         if not begin_test
         then E.Ignore
         else (
           let expect result =
             E.of_thunk (fun () ->
               Result.ok_or_failwith
                 (Result.map_error result ~f:(fun error ->
                    Sexp.to_string (Input.Command_error.sexp_of_t error))))
           in
           let%bind initial =
             Controller.replace editor ~selection:End ~undo:Reset "é界" >>= expect
           in
           let selection = Input.Selection.create ~anchor:5 ~head:2 |> Or_error.ok_exn in
           let%bind selected = Controller.select editor selection >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               assert (Input.Selection.equal (Input.Snapshot.selection selected) selection))
           in
           let%bind _ =
             Controller.replace editor ~selection:Start ~undo:Record "changed" >>= expect
           in
           let%bind restored = Controller.command editor Undo >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               assert (String.equal (Input.Snapshot.text restored) "é界");
               assert (Input.Selection.equal (Input.Snapshot.selection restored) selection))
           in
           let%bind redone = Controller.command editor Redo >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               let start = Input.Selection.create ~anchor:0 ~head:0 |> Or_error.ok_exn in
               assert (Input.Selection.equal (Input.Snapshot.selection redone) start))
           in
           let%bind stale =
             Controller.replace
               editor
               ~if_revision:(Input.Snapshot.revision initial)
               ~selection:Start
               ~undo:Record
               ""
           in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Input.Snapshot.equal
                   Input.Command_error.equal
                   stale
                   (Error Stale_revision)))
           in
           let%bind cleared =
             Controller.replace editor ~selection:Start ~undo:Record "" >>= expect
           in
           let%bind () =
             E.of_thunk (fun () -> assert (String.is_empty (Input.Snapshot.text cleared)))
           in
           let%bind () = set_shown false in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind stale = Controller.focus editor in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Input.Snapshot.equal
                   Input.Command_error.equal
                   stale
                   (Error Stale_editor)))
           in
           E.of_thunk (fun () ->
             incr completed;
             App.Window.close window)))
    graph;
  let%arr editor = editor
  and submission = submission
  and shown = shown in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 16.); Gap (Gpuio.Length.px_exn 12.) ])
    ([ View.text submission ]
     @ (if shown
        then
          [ Controller.view
              ~style:
                (Gpuio.Style.create_exn
                   [ Width (Gpuio.Length.px_exn 400.)
                   ; Padding (Gpuio.Length.px_exn 8.)
                   ; Border_width 1.
                   ; Border_color (Gpuio.Color.rgb_exn 0x888888)
                   ])
              editor
          ]
        else [])
     @ [ View.button ~on_click:(E.of_thunk (fun () -> App.Window.close window)) "Close" ]
    )
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref 0 in
  App.run (fun _ app ->
    List.iter [ Input.Mode.Single_line; Multiline ] ~f:(fun mode ->
      App.open_window
        app
        ~title:"GPUIO native text input"
        ~width:460.
        ~height:320.
        (component ~self_test ~completed ~mode)
      |> Or_error.ok_exn
      |> fun (_ : App.Window.t) -> ()));
  if self_test
  then (
    assert (!completed = 2);
    print_endline
      "GPUIO_EDITOR_PUBLIC_OK: two windows, UTF-8 selection, revision guard, undo/redo, \
       stale unmount, close")
;;
