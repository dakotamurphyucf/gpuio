open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Input = Gpuio_eio.Text_input
module B = Bonsai.Cont
module E = Bonsai.Effect
module UI = Gpuio_bonsai.View
module Q = Container_query

let ok = Or_error.ok_exn
let compact = Q.Branch_id.of_string "compact" |> ok
let wide = Q.Branch_id.of_string "wide" |> ok

let config =
  Q.Config.create
    ~default:compact
    [ Q.Rule.create
        ~branch:wide
        ~condition:(Q.Predicate.create ~width:(Q.Range.create ~minimum:600. () |> ok) ())
    ]
  |> ok
;;

let style = Style.create_exn
let px = Length.px_exn

let branch ~name ~accent ~editors ~activated ~deactivated window graph =
  B.Edge.lifecycle
    ~on_activate:(B.return (E.of_thunk (fun () -> incr activated)))
    ~on_deactivate:(B.return (E.of_thunk (fun () -> incr deactivated)))
    graph;
  let editor =
    Input.create
      window
      ~initial_text:(name ^ " draft")
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:(name ^ " draft") () |> ok))
      graph
  in
  let count, set_count = B.state 0 graph in
  let open B.Let_syntax in
  B.Edge.after_display
    (let%arr editor = editor in
     E.of_thunk (fun () -> editors := Map.set !editors ~key:name ~data:editor))
    graph;
  let%arr editor = editor
  and count = count
  and set_count = set_count in
  UI.column
    ~style:
      (style
         [ Width (Length.percent_exn 100.)
         ; Height (Length.percent_exn 100.)
         ; Padding (px 28.)
         ; Gap (px 18.)
         ; Foreground (Color.rgb_exn 0xe8ecf5)
         ; Background (Background.solid (Color.rgb_exn 0x141923))
         ])
    [ UI.text
        ~style:(style [ Foreground (Color.rgb_exn accent); Font_size 13. ])
        "GPUIO / RESPONSIVE WORKSPACE"
    ; UI.text ~style:(style [ Font_size 30. ]) (name ^ " presentation")
    ; UI.text "Resize this window across 600 logical pixels. The choice happens natively."
    ; UI.column
        ~style:
          (style
             [ Padding (px 20.)
             ; Gap (px 14.)
             ; Radius 16.
             ; Background (Background.solid (Color.rgb_exn 0x202838))
             ])
        [ UI.text "Each presentation keeps its own draft and counter."
        ; Input.view
            ~style:(style [ Height (px 40.); Width (Length.percent_exn 100.) ])
            editor
        ; UI.button
            ~style:
              (style
                 [ Padding (px 12.)
                 ; Radius 8.
                 ; Background (Background.solid (Color.rgb_exn accent))
                 ; Foreground (Color.rgb_exn 0x10151f)
                 ])
            ~on_click:(set_count (count + 1))
            (sprintf "Local count · %d" count)
        ]
    ; UI.text "Native visibility changes do not deactivate the Bonsai computation."
    ]
;;

let component ~editors ~activated ~deactivated ~events window graph =
  let compact_view =
    branch ~name:"Compact" ~accent:0x86adff ~editors ~activated ~deactivated window graph
  in
  let wide_view =
    branch ~name:"Wide" ~accent:0x79d5b0 ~editors ~activated ~deactivated window graph
  in
  let open B.Let_syntax in
  let%arr compact_view = compact_view
  and wide_view = wide_view in
  UI.container_query
    ~on_select:(fun selection -> E.of_thunk (fun () -> events := selection :: !events))
    config
    [ compact, compact_view; wide, wide_view ]
  |> ok
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false
  and activated = ref 0
  and deactivated = ref 0 in
  App.run (fun env app ->
    let editors = ref String.Map.empty
    and events = ref [] in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO responsive workspace"
        ~width:520.
        ~height:360.
        (component ~editors ~activated ~deactivated ~events)
      |> ok
    in
    if self_test
    then
      Scope.start
        (App.Window.scope window)
        ~f:(fun () ->
          let clock = Eio.Stdenv.clock env in
          Eio.Time.with_timeout_exn clock 20. (fun () ->
            let await label f =
              Eio.traceln "query self-test: waiting for %s" label;
              while not (f ()) do
                Eio.Time.sleep clock 0.005
              done
            in
            let on_ui ui_effect =
              let promise, resolver = Eio.Promise.create () in
              Scope.Expert.enqueue (App.Window.scope window) (fun () ->
                E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
              Eio.Promise.await promise
            in
            let editor name = Map.find_exn !editors name in
            let read name =
              match on_ui (Input.read_snapshot (editor name)) with
              | Ok snapshot -> snapshot
              | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]
            in
            let selected branch =
              Option.exists (List.hd !events) ~f:(fun s ->
                Q.Branch_id.equal s.Q.Selection.branch branch)
            in
            let resize width =
              (match on_ui (App.Window.command window (Resize (width, 360.))) with
               | Ok _ -> ()
               | Error error -> raise_s [%sexp (error : Window.Error.t)]);
              await "window resize observation" (fun () ->
                Option.exists (App.Window.snapshot window) ~f:(fun s ->
                  Float.(abs (s.content_width -. width) < 1.)))
            in
            await "initial selection and editors" (fun () ->
              Map.length !editors = 2
              && Map.for_all !editors ~f:(fun editor ->
                Option.is_some (Input.snapshot editor))
              && selected compact);
            assert (!activated = 2 && !deactivated = 0);
            let replacement =
              on_ui
                (Input.replace
                   (editor "Compact")
                   ~selection:End
                   ~undo:Record
                   "A retained Unicode draft 👩🏽‍💻")
            in
            (match replacement with
             | Ok _ -> ()
             | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]);
            let before = read "Compact" in
            resize 760.;
            await "wide selection" (fun () -> selected wide);
            let after = read "Compact" in
            assert (Text_input.Snapshot.equal before after);
            assert (
              Result.equal
                Text_input.Snapshot.equal
                Text_input.Command_error.equal
                (on_ui (Input.focus (editor "Compact")))
                (Error Focus_blocked));
            assert (!activated = 2 && !deactivated = 0);
            let event_count = List.length !events in
            let commits = (App.stats app).commits in
            resize 800.;
            Eio.Time.sleep clock 0.12;
            assert (List.length !events = event_count);
            assert ((App.stats app).commits = commits);
            resize 520.;
            await "compact selection" (fun () -> selected compact);
            assert (
              String.equal
                (Text_input.Snapshot.text (read "Compact"))
                "A retained Unicode draft 👩🏽‍💻");
            assert (!activated = 2 && !deactivated = 0);
            completed := true))
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            ok result;
            App.Window.close window))
      |> ok
      |> fun (_ : Scope.Task.t) -> ());
  if self_test
  then (
    assert (!completed && !activated = 2 && !deactivated = 2);
    Eio.traceln
      "GPUIO_CONTAINER_QUERY_PUBLIC_OK: native resize selection, retained hidden editor, \
       focus denial, silent same-branch resize, Bonsai lifecycle and scoped shutdown")
;;
