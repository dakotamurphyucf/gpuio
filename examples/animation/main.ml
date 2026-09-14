open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module UI = Gpuio_bonsai.View
module Animation = Gpuio.Animation

let target width opacity radius =
  Animation.Target.create [ Width, width; Opacity, opacity; Radius, radius ]
  |> Or_error.ok_exn
;;

let component ~phase_var ~events ~ready _window graph =
  B.Edge.lifecycle ~on_activate:(B.return (E.of_thunk (fun () -> ready := true))) graph;
  let open B.Let_syntax in
  let%arr phase = B.Expert.Var.value phase_var in
  let width = if phase % 2 = 0 then 240. else 72. in
  let config =
    Animation.Config.create
      ~initial:(target 0. 0. 0.)
      ~target:(target width 1. 12.)
      ~duration:(Time_ns.Span.of_ms 220.)
      ~easing:Animation.Easing.ease_in_out
      ()
    |> Or_error.ok_exn
  in
  let open Gpuio in
  UI.column
    ~style:
      (Style.create_exn
         [ Padding (Length.px_exn 24.)
         ; Gap (Length.px_exn 12.)
         ; Foreground (Color.token_exn "foreground")
         ; Background (Background.solid (Color.token_exn "background"))
         ])
    [ UI.button
        ~on_click:(E.of_thunk (fun () -> B.Expert.Var.set phase_var (phase + 1)))
        "Toggle sidebar"
    ; UI.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 16.) ])
        [ UI.animate
            ~key:(Key.of_string_exn "sidebar")
            config
            ~style:
              (Style.create_exn
                 [ Height (Length.px_exn 120.)
                 ; Shrink 0.
                 ; Overflow_x Hidden
                 ; Overflow_y Hidden
                 ; Background (Background.solid (Color.token_exn "accent"))
                 ])
            ~on_event:(fun event -> E.of_thunk (fun () -> events := event :: !events))
            [ UI.text
                ~style:
                  (Style.create_exn
                     [ Width (Length.px_exn 240.)
                     ; Shrink 0.
                     ; Padding (Length.px_exn 12.)
                     ])
                "A fixed-width sidebar. This text keeps its layout while the outer panel \
                 reveals or clips it."
            ]
        ; UI.column
            [ UI.text "Native motion"
            ; UI.text "Bonsai declares targets; Rust renders every animation frame."
            ]
        ]
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let phase_var = B.Expert.Var.create 0 in
    let events = ref [] in
    let ready = ref false in
    let window =
      App.open_window
        app
        ~title:"GPUIO native animation"
        ~width:640.
        ~height:260.
        (component ~phase_var ~events ~ready)
      |> Or_error.ok_exn
    in
    if self_test
    then
      Scope.start
        (App.Window.scope window)
        ~f:(fun () ->
          let clock = Eio.Stdenv.clock env in
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            let rec await predicate =
              if not (predicate ())
              then (
                Eio.Time.sleep clock 0.005;
                await predicate)
            in
            await (fun () -> !ready);
            let frame, resolver = Eio.Promise.create () in
            App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
              E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
            |> Or_error.ok_exn;
            Eio.Promise.await frame;
            B.Expert.Var.set phase_var 1;
            let found run outcome =
              List.exists !events ~f:(fun event ->
                Int64.equal (Animation.Run_id.to_int64 event.run_id) run
                && Animation.Outcome.equal event.outcome outcome)
            in
            await (fun () ->
              (found 1L (Cancelled Replaced) || found 1L Finished) && found 2L Finished);
            let theme =
              Gpuio.Theme.create
                [ "background", Gpuio.Color.rgb_exn 0xf8fafc
                ; "foreground", Gpuio.Color.rgb_exn 0x172136
                ; "accent", Gpuio.Color.rgb_exn 0x77aaff
                ; "muted", Gpuio.Color.rgb_exn 0x556677
                ]
              |> Or_error.ok_exn
            in
            App.Window.set_theme window theme;
            B.Expert.Var.set phase_var 2;
            await (fun () -> found 3L Finished);
            assert (List.length !events = 3);
            completed := true))
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            Or_error.ok_exn result;
            App.Window.close window))
      |> Or_error.ok_exn
      |> fun (_ : Scope.Task.t) -> ());
  if self_test
  then (
    assert !completed;
    Eio.traceln
      "GPUIO_ANIMATION_PUBLIC_OK: Bonsai targets, native endpoints, theme update and \
       clean shutdown")
;;
