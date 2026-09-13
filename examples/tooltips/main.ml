open Core
module B = Bonsai.Cont
module View = Gpuio_bonsai.View
module App = Gpuio_eio.App

let component ~self_test ~completed window graph =
  let open_, set_open = B.state false graph in
  let mounted, set_mounted = B.state true graph in
  let input =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create ~mode:Single_line ~label:"Tooltip note" ()
            |> Or_error.ok_exn))
      graph
  in
  let sleep = B.Clock.sleep graph in
  let started = ref false in
  let open B.Let_syntax in
  B.Edge.on_change
    (B.map input ~f:Gpuio_eio.Text_input.snapshot)
    ~equal:(Option.equal Gpuio.Text_input.Snapshot.equal)
    ~callback:
      (let%arr input = input
       and set_open = set_open
       and set_mounted = set_mounted
       and sleep = sleep in
       fun observation ->
         let module E = Bonsai.Effect in
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
                    Sexp.to_string (Gpuio.Text_input.Command_error.sexp_of_t error))))
           in
           let expect_error result error =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Gpuio.Text_input.Snapshot.equal
                   Gpuio.Text_input.Command_error.equal
                   result
                   (Error error)))
           in
           let%bind blocked = Gpuio_eio.Text_input.focus input in
           let%bind () = expect_error blocked Focus_blocked in
           let%bind changed =
             Gpuio_eio.Text_input.replace input ~selection:End ~undo:Reset "Retained é界"
             >>= expect
           in
           let%bind () = set_open true in
           let%bind () = sleep (Time_ns.Span.of_ms 100.) in
           let%bind shown = Gpuio_eio.Text_input.focus input >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               let module S = Gpuio.Text_input.Snapshot in
               assert (String.equal (S.text changed) (S.text shown));
               assert (
                 Gpuio.Text_input.Revision.equal (S.revision changed) (S.revision shown));
               assert (
                 Gpuio.Text_input.Selection.equal
                   (S.selection changed)
                   (S.selection shown));
               assert (S.focused shown))
           in
           let%bind () = set_open false in
           let%bind () = sleep (Time_ns.Span.of_ms 100.) in
           let%bind blocked = Gpuio_eio.Text_input.focus input in
           let%bind () = expect_error blocked Focus_blocked in
           let%bind () = set_open true in
           let%bind () = sleep (Time_ns.Span.of_ms 100.) in
           let%bind shown = Gpuio_eio.Text_input.focus input >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               let module S = Gpuio.Text_input.Snapshot in
               assert (String.equal (S.text changed) (S.text shown));
               assert (
                 Gpuio.Text_input.Revision.equal (S.revision changed) (S.revision shown));
               assert (
                 Gpuio.Text_input.Selection.equal
                   (S.selection changed)
                   (S.selection shown));
               assert (S.focused shown))
           in
           let%bind () = set_mounted false in
           let%bind () = sleep (Time_ns.Span.of_ms 100.) in
           let%bind stale = Gpuio_eio.Text_input.focus input in
           let%bind () = expect_error stale Stale_editor in
           E.of_thunk (fun () ->
             completed := true;
             App.Window.close window)))
    graph;
  let%arr open_ = open_
  and set_open = set_open
  and mounted = mounted
  and input = input in
  let config =
    Gpuio.Tooltip.Config.create
      ~label:"Editable tooltip note"
      ~open_state:(Controlled open_)
      ()
    |> Or_error.ok_exn
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 16.) ])
    [ View.text "Hover or focus the anchors. Escape dismisses a tooltip."
    ; View.tooltip
        ~config:
          (Gpuio.Tooltip.Config.create ~label:"Model information" () |> Or_error.ok_exn)
        ~anchor:(View.button ~on_click:Bonsai.Effect.Ignore "Model information")
        ~content:(View.text "This tooltip manages hover and focus in the native runtime.")
        ()
    ; View.checkbox
        ~state:(if open_ then Checked else Unchecked)
        ~on_toggle:(set_open (not open_))
        "Show editable note"
    ; (if mounted
       then
         View.tooltip
           ~key:(Gpuio.Key.of_string_exn "note")
           ~config
           ~on_open_change:set_open
           ~anchor:(View.button ~on_click:Bonsai.Effect.Ignore "Editable note")
           ~content:
             (View.column
                [ View.text "Your text survives closing and reopening."
                ; Gpuio_eio.Text_input.view input
                ])
           ()
       else View.text "Tooltip unmounted")
    ; View.button
        ~on_click:(Bonsai.Effect.of_thunk (fun () -> App.Window.close window))
        "Quit"
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    App.open_window
      app
      ~title:"GPUIO tooltips"
      ~width:520.
      ~height:400.
      (component ~self_test ~completed)
    |> Or_error.ok_exn
    |> fun (_ : App.Window.t) -> ());
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_TOOLTIPS_PUBLIC_OK: controlled visibility, hidden focus denial, retained \
       editor snapshot, stale unmount and clean shutdown")
;;
