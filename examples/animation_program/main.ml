open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module UI = Gpuio_bonsai.View
module A = Animation

let ok = Or_error.ok_exn
let target width = A.Target.create [ Width, width ] |> ok
let stage timing width = A.Stage.create ~timing ~target:(target width) () |> ok
let tween ms = A.Timing.tween (Time_ns.Span.of_ms ms) |> ok
let spring = A.Spring.create ~stiffness:140. ~damping:18. ~mass:1. () |> ok

let sequence =
  A.Program.create
    ~initial:(target 24.)
    [ stage (tween 300.) 140.; stage (A.Timing.spring spring) 280. ]
  |> ok
;;

let shared =
  A.Program.create
    ~initial:(target 40.)
    ~repeat:Alternate
    ~clock:(A.Clock.group "activity" |> ok)
    [ stage (tween 700.) 160. ]
  |> ok
;;

let card_style color =
  Style.create_exn
    [ Height (Length.px_exn 36.)
    ; Shrink 0.
    ; Radius 10.
    ; Background (Background.solid (Color.rgb_exn color))
    ]
;;

let component ~program_var ~members_var ~ready ~events ~repeat_events _window graph =
  B.Edge.lifecycle ~on_activate:(B.return (E.of_thunk (fun () -> ready := true))) graph;
  let open B.Let_syntax in
  let%arr program = B.Expert.Var.value program_var
  and members = B.Expert.Var.value members_var in
  let change f = E.of_thunk (fun () -> B.Expert.Var.set program_var (f program)) in
  UI.column
    ~style:
      (Style.create_exn
         [ Padding (Length.px_exn 28.)
         ; Gap (Length.px_exn 18.)
         ; Foreground (Color.token_exn "foreground")
         ; Background (Background.solid (Color.token_exn "background"))
         ])
    [ UI.text "Native motion programs"
    ; UI.text
        "An ordered reveal followed by a physical spring. Every frame stays in Rust."
    ; UI.animate_program
        ~key:(Key.of_string_exn "sequence")
        program
        []
        ~style:(card_style 0x528bff)
        ~on_event:(fun event -> E.of_thunk (fun () -> events := event :: !events))
    ; UI.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.) ])
        [ UI.button "Pause" ~on_click:(change (fun p -> A.Program.with_playback p Paused))
        ; UI.button
            "Resume"
            ~on_click:(change (fun p -> A.Program.with_playback p Running))
        ; UI.button "Restart" ~on_click:(change (fun p -> A.Program.restart p |> ok))
        ; UI.button "Reverse" ~on_click:(change (fun p -> A.Program.reverse p |> ok))
        ; UI.button
            "Cancel"
            ~on_click:(change (fun p -> A.Program.with_playback p Cancelled))
        ]
    ; UI.text "Shared activity phase — new members join the running clock."
    ; UI.column
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.) ])
        (List.init members ~f:(fun index ->
           UI.animate_program
             ~key:(Key.of_string_exn (Int.to_string index))
             shared
             []
             ~style:(card_style (if index = 0 then 0x8a73e8 else 0x57b69d))
             ~on_event:(fun event ->
               E.of_thunk (fun () -> repeat_events := event :: !repeat_events))))
    ; UI.button
        (if members = 1 then "Join a second member" else "Remove second member")
        ~on_click:
          (E.of_thunk (fun () ->
             B.Expert.Var.set members_var (if members = 1 then 2 else 1)))
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run
    ~motion:(if self_test then Full else System)
    (fun env app ->
       let program_var = B.Expert.Var.create sequence in
       let members_var = B.Expert.Var.create 1 in
       let ready = ref false
       and events = ref []
       and repeat_events = ref [] in
       let window =
         App.open_window
           app
           ~title:"GPUIO motion programs"
           ~width:680.
           ~height:420.
           (component ~program_var ~members_var ~ready ~events ~repeat_events)
         |> ok
       in
       if self_test
       then
         Scope.start
           (App.Window.scope window)
           ~f:(fun () ->
             let clock = Eio.Stdenv.clock env in
             Eio.Time.with_timeout_exn clock 20. (fun () ->
               let rec await f =
                 if not (f ())
                 then (
                   Eio.Time.sleep clock 0.005;
                   await f)
               in
               let frame () =
                 let promise, resolver = Eio.Promise.create () in
                 App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
                   E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
                 |> ok;
                 Eio.Promise.await promise
               in
               let found run =
                 List.exists !events ~f:(fun event ->
                   Int64.equal (A.Run_id.to_int64 event.A.Program.Event.run_id) run
                   && List.exists
                        event.observations
                        ~f:(A.Program.Observation.equal Finished))
               in
               await (fun () -> !ready);
               frame ();
               B.Expert.Var.set program_var (A.Program.with_playback sequence Paused);
               frame ();
               let before = List.length !events in
               Eio.Time.sleep clock 0.2;
               assert (List.length !events = before);
               B.Expert.Var.set program_var sequence;
               await (fun () -> found 1L);
               B.Expert.Var.set members_var 2;
               frame ();
               Eio.Time.sleep clock 0.1;
               assert (List.is_empty !repeat_events);
               App.set_motion app Reduce;
               B.Expert.Var.set program_var (A.Program.restart sequence |> ok);
               await (fun () -> found 4L);
               let reduced =
                 List.find_exn !events ~f:(fun event ->
                   Int64.equal (A.Run_id.to_int64 event.run_id) 4L)
               in
               assert (
                 List.equal
                   A.Program.Observation.equal
                   reduced.observations
                   [ Stage_completed (0, Reduced_motion)
                   ; Stage_completed (1, Reduced_motion)
                   ; Finished
                   ]);
               assert (List.is_empty !repeat_events);
               completed := true))
           ~on_result:(fun result ->
             E.of_thunk (fun () ->
               ok result;
               App.Window.close window))
         |> ok
         |> fun (_ : Scope.Task.t) -> ());
  if self_test
  then (
    assert !completed;
    Eio.traceln
      "GPUIO_ANIMATION_PROGRAM_PUBLIC_OK: Bonsai pause/resume, preserved run, reduced \
       stage batch, shared repeat without frame callbacks and scoped shutdown")
;;
