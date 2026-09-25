open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module S = Gpuio.Slider
module Controller = Gpuio_eio.Slider
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let range lower upper = S.Value.range ~lower ~upper |> ok
let domain = Gpuio.Numeric.Domain.create ~min:(-2.) ~max:8. ~step:0.5 |> ok

let expect = function
  | Ok value -> E.return value
  | Error error ->
    E.of_thunk (fun () -> failwith (Sexp.to_string (S.Command_error.sexp_of_t error)))
;;

let assert_error result expected =
  E.of_thunk (fun () ->
    assert (Result.equal S.Snapshot.equal S.Command_error.equal result (Error expected)))
;;

let assert_value snapshot value =
  E.of_thunk (fun () -> assert (S.Value.equal (S.Snapshot.value snapshot) value))
;;

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let shown, set_shown = B.state true graph in
  let disabled, set_disabled = B.state false graph in
  let read_only, set_read_only = B.state false graph in
  let status, set_status = B.state "Ready" graph in
  let config =
    let%arr disabled = disabled
    and read_only = read_only in
    S.Config.create
      ~domain
      ~label:"Output range"
      ~lower_label:"Minimum output"
      ~upper_label:"Maximum output"
      ~disabled
      ~read_only
      ()
    |> ok
  in
  let slider = Controller.create window ~config ~initial:(range 2. 7.) graph in
  let single =
    Controller.create
      window
      ~config:(B.return (S.Config.create ~domain ~label:"Single linear value" () |> ok))
      ~initial:(S.Value.single 3. |> ok)
      graph
  in
  let logarithmic_domain =
    Gpuio.Numeric.Domain.create ~min:1. ~max:1000. ~step:1. |> ok
  in
  let logarithmic_config label =
    S.Config.create ~domain:logarithmic_domain ~label ~axis:Vertical ~scale:Logarithmic ()
    |> ok
    |> B.return
  in
  let vertical_single =
    Controller.create
      window
      ~config:(logarithmic_config "Logarithmic value")
      ~initial:(S.Value.single 32. |> ok)
      graph
  in
  let vertical_range =
    Controller.create
      window
      ~config:(logarithmic_config "Logarithmic interval")
      ~initial:(range 10. 100.)
      graph
  in
  let observed = B.map slider ~f:Controller.snapshot in
  let sleep = B.Clock.sleep graph in
  (* Test-only references keep the deliberate old lease while new observations
     arrive. Application state is carried by the controller/Bonsai model. *)
  let started = ref false in
  let latest = ref None in
  B.Edge.on_change
    observed
    ~equal:(Option.equal S.Snapshot.equal)
    ~callback:
      (let%arr slider = slider
       and sleep = sleep
       and set_shown = set_shown
       and set_disabled = set_disabled in
       fun observation ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             latest := Some slider;
             if self_test && Option.is_some observation && not !started
             then (
               started := true;
               true)
             else false)
         in
         if not begin_test
         then E.Ignore
         else (
           let%bind initial = Controller.read_snapshot slider >>= expect in
           let%bind () = assert_value initial (range 2. 7.) in
           let%bind next =
             Controller.replace_if_unchanged slider initial (range 0. 6.) >>= expect
           in
           let%bind () = assert_value next (range 0. 6.) in
           let%bind stale =
             Controller.replace_if_unchanged slider initial (range 1. 5.)
           in
           let%bind () = assert_error stale Stale_revision in
           let%bind wrong = Controller.replace slider (S.Value.single 1. |> ok) in
           let%bind () = assert_error wrong Wrong_mode in
           let%bind wrong_thumb = Controller.focus slider Single in
           let%bind () = assert_error wrong_thumb Wrong_thumb in
           let%bind _ = Controller.focus slider Lower >>= expect in
           let%bind unchanged = Controller.cancel_drag slider >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 S.Revision.equal
                   (S.Snapshot.revision unchanged)
                   (S.Snapshot.revision next)))
           in
           let%bind () = set_disabled true in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind denied = Controller.focus slider Upper in
           let%bind () = assert_error denied Disabled in
           let%bind replaced = Controller.replace slider (range 1. 4.) >>= expect in
           let%bind () = assert_value replaced (range 1. 4.) in
           let%bind read = Controller.read_snapshot slider >>= expect in
           let%bind () = assert_value read (range 1. 4.) in
           let%bind () = set_shown false in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind stale = Controller.read_snapshot slider in
           let%bind () = assert_error stale Stale_slider in
           let%bind () = set_disabled false in
           let%bind () = set_shown true in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind stale = Controller.read_snapshot slider in
           let%bind () = assert_error stale Stale_slider in
           let%bind current = E.of_thunk (fun () -> Option.value_exn !latest) in
           let%bind mounted = Controller.read_snapshot current >>= expect in
           let%bind () = assert_value mounted (range 2. 7.) in
           let%bind stale =
             Controller.replace_if_unchanged current initial (range 0. 1.)
           in
           let%bind () = assert_error stale Stale_slider in
           let%bind () = E.of_thunk (fun () -> App.Window.close window) in
           let%bind closed = Controller.read_snapshot current in
           let%bind () = assert_error closed Closed in
           E.of_thunk (fun () -> completed := true)))
    graph;
  let%arr slider = slider
  and shown = shown
  and set_shown = set_shown
  and disabled = disabled
  and set_disabled = set_disabled
  and read_only = read_only
  and set_read_only = set_read_only
  and status = status
  and set_status = set_status
  and single = single
  and vertical_single = vertical_single
  and vertical_range = vertical_range in
  let report pending =
    let open E.Let_syntax in
    let%bind result = pending in
    set_status
      (match result with
       | Ok snapshot ->
         "Revision "
         ^ Int64.to_string (S.Revision.to_int64 (S.Snapshot.revision snapshot))
       | Error error -> Sexp.to_string (S.Command_error.sexp_of_t error))
  in
  let snapshot = Controller.snapshot slider in
  let value =
    Option.value_map snapshot ~default:"Waiting for native mount" ~f:(fun snapshot ->
      Sexp.to_string (S.Value.sexp_of_t (S.Snapshot.value snapshot)))
  in
  let mode_view title controller =
    let description =
      Option.value_map
        (Controller.snapshot controller)
        ~default:"Mounting"
        ~f:(fun snapshot ->
          Sexp.to_string (S.Value.sexp_of_t (S.Snapshot.value snapshot)))
    in
    View.column
      ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
      [ View.text title
      ; Controller.view
          ~style:(Gpuio.Style.create_exn [ Foreground (Gpuio.Color.rgb_exn 0x48b5a0) ])
          controller
      ; View.text description
      ]
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 16.) ])
    ([ View.text "Native numeric controls"; View.text "Output range" ]
     @ (if shown
        then
          [ Controller.view
              ~style:(Gpuio.Style.create_exn [ Width (Gpuio.Length.px_exn 320.) ])
              slider
          ]
        else [])
     @ [ View.text value
       ; View.text status
       ; View.row
           ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
           [ View.button
               ~on_click:(report (Controller.replace slider (range 0. 6.)))
               "Reset range"
           ; View.button ~on_click:(report (Controller.read_snapshot slider)) "Read state"
           ; View.button
               ~on_click:(report (Controller.focus slider Lower))
               "Focus minimum"
           ]
       ; View.row
           ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
           [ View.button
               ~on_click:(set_disabled (not disabled))
               (if disabled then "Enable" else "Disable")
           ; View.button
               ~on_click:(set_read_only (not read_only))
               (if read_only then "Allow edits" else "Read-only")
           ; View.button
               ~on_click:(set_shown (not shown))
               (if shown then "Unmount" else "Mount")
           ; View.button
               ~on_click:(E.of_thunk (fun () -> App.Window.close window))
               "Close"
           ]
       ; mode_view "Single · linear · horizontal" single
       ; View.row
           ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 40.) ])
           [ mode_view "Single · logarithmic · vertical" vertical_single
           ; mode_view "Range · logarithmic · vertical" vertical_range
           ]
       ])
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    ignore
      (App.open_window
         app
         ~title:"GPUIO numeric controls"
         ~width:720.
         ~height:720.
         (component ~self_test ~completed)
       |> ok
       : App.Window.t));
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_SLIDER_PUBLIC_OK: native observations, correlated commands, revision/lease \
       errors, disabled replacement, remount and close")
;;
