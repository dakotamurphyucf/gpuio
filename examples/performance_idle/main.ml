open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Editor = Gpuio_eio.Text_input
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Probe = Gpuio_performance_probe

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component ~command ~events ~observed window graph =
  let open B.Let_syntax in
  let editor =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Idle editor" () |> ok))
      ~initial_text:"Focus this editor, then move focus to the settle button."
      graph
  in
  B.Edge.after_display
    (let%arr editor = editor in
     E.of_thunk (fun () -> observed := Some editor))
    graph;
  let%arr editor = editor
  and sequence, request = B.Expert.Var.value command in
  V.column
    ~style:
      (style
         [ Padding (px 24.)
         ; Gap (px 16.)
         ; Background (Background.solid (Color.rgb_exn 0xf5f6f8))
         ; Foreground (Color.rgb_exn 0x182030)
         ])
    [ V.text "GPUIO qualification · settled focused and unfocused idle"
    ; Editor.view ~style:(style [ Width (px 700.); Height (px 40.) ]) editor
    ; V.button ~on_click:E.Ignore "Settle without a caret"
    ; V.text "The test observes an exposed window without requesting frames."
    ; V.extension
        (Probe.instance ~sequence request |> ok)
        ~on_event:(function
          | Extension.Event.Data event ->
            E.of_thunk (fun () -> Queue.enqueue events event)
          | Failed error ->
            E.of_thunk (fun () -> raise_s [%sexp (error : Extension.Error.t)])
          | Mounted | Command_completed _ -> E.Ignore)
    ]
;;

let run ~smoke =
  let seconds = if smoke then 2 else 60 in
  App.run ~exit_on_last_window:false (fun env app ->
    let scope = App.scope app in
    let clock = Eio.Stdenv.clock env in
    let input = Eio.Buf_read.of_flow (Eio.Stdenv.stdin env) ~max_size:128 in
    let command = B.Expert.Var.create (1L, Probe.Command.Document_preparation) in
    let events = Queue.create () in
    let observed = ref None in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO · Idle qualification"
        ~width:1200.
        ~height:800.
        (component ~command ~events ~observed)
      |> ok
    in
    let active () = Option.map (App.Window.snapshot window) ~f:(fun s -> s.active) in
    let previous = ref (active ()) in
    let changes = ref 0 in
    App.Window.on_change window (fun snapshot ->
      E.of_thunk (fun () ->
        let next = Some snapshot.active in
        if not (Option.equal Bool.equal next !previous) then Int.incr changes;
        previous := next));
    let perform ui_effect =
      let promise, resolver = Eio.Promise.create () in
      ignore
        (Scope.start
           scope
           ~f:(fun () -> ())
           ~on_result:(fun result ->
             ok result;
             E.map ui_effect ~f:(Eio.Promise.resolve resolver))
         |> ok
         : Scope.Task.t);
      Eio.Promise.await promise
    in
    let sync f = perform (E.of_thunk f) in
    let emit name value =
      Eio.Flow.copy_string
        ("GPUIO_IDLE_PERF " ^ name ^ " " ^ Sexp.to_string value ^ "\n")
        (Eio.Stdenv.stdout env)
    in
    let wait f =
      Eio.Time.with_timeout_exn clock 20. (fun () ->
        while not (f ()) do
          Eio.Time.sleep clock 0.01
        done)
    in
    let next () =
      wait (fun () -> not (Queue.is_empty events));
      Queue.dequeue_exn events
    in
    let sequence = ref 1L in
    let send request =
      Int64.incr sequence;
      sync (fun () -> B.Expert.Var.set command (!sequence, request))
    in
    let work () =
      emit "config" [%sexp (smoke : bool), (seconds : int)];
      wait (fun () -> Option.is_some !observed);
      (match next () with
       | Probe.Event.Document_preparation _ -> ()
       | event -> raise_s [%sexp (event : Probe.Event.t)]);
      List.iter
        [ "focused", true; "unfocused", false ]
        ~f:(fun (phase, expected) ->
          emit "ready" [%sexp (phase : string)];
          assert (
            String.equal
              (Eio.Time.with_timeout_exn clock 20. (fun () -> Eio.Buf_read.line input))
              ("ready " ^ phase));
          wait (fun () ->
            Option.value_map (active ()) ~default:false ~f:(Bool.equal expected));
          let editor = Option.value_exn !observed in
          let snapshot =
            match perform (Editor.read_snapshot editor) with
            | Ok snapshot -> snapshot
            | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]
          in
          assert (not (Text_input.Snapshot.focused snapshot));
          let focused = perform (App.Window.focused_input window) in
          (match focused with
           | Ok None -> ()
           | _ -> failwith "Native input remained focused");
          send Probe.Command.Begin_idle;
          (match next () with
           | Probe.Event.Begun ns -> emit "begin" [%sexp (phase : string), (ns : int64)]
           | event -> raise_s [%sexp (event : Probe.Event.t)]);
          let before, count = sync (fun () -> active (), !changes) in
          emit "state-before" [%sexp (phase : string), (before : bool option)];
          Eio.Time.sleep clock (Float.of_int seconds);
          send Probe.Command.Finish;
          let finished = next () in
          let counts =
            match finished with
            | Probe.Event.Finished { counts; _ } -> counts
            | event -> raise_s [%sexp (event : Probe.Event.t)]
          in
          emit "finish" [%sexp (phase : string), (finished : Probe.Event.t)];
          (match next () with
           | Probe.Event.Idle_observations observations ->
             emit
               "observations"
               [%sexp
                 (phase : string), (observations : (int64 * bool * bool option) list)]
           | event -> raise_s [%sexp (event : Probe.Event.t)]);
          List.iteri counts ~f:(fun metric _ ->
            send (Probe.Command.Buckets { metric; offset = 0 });
            emit "buckets" [%sexp (phase : string), (next () : Probe.Event.t)]);
          let after, change_count = sync (fun () -> active (), !changes - count) in
          emit
            "state-after"
            [%sexp (phase : string), (after : bool option), (change_count : int)];
          assert (Option.equal Bool.equal before (Some expected));
          assert (Option.equal Bool.equal after before && change_count = 0);
          assert (List.for_all counts ~f:(Int64.equal 0L)));
      sync (fun () ->
        App.Window.close window;
        observed := None);
      wait (fun () ->
        let d = App.diagnostics app in
        d.windows = 0
        && d.pending_requests = 0
        && d.queued_jobs = 0
        && d.queued_commands = 0);
      emit "cleanup" [%sexp (App.diagnostics app : App.Diagnostics.t)];
      emit "complete" [%sexp (2 : int)];
      App.shutdown app
    in
    ignore
      (Scope.start scope ~f:work ~on_result:(fun result ->
         E.of_thunk (fun () -> ok result))
       |> ok
       : Scope.Task.t))
;;

let () = run ~smoke:(Array.exists (Sys.get_argv ()) ~f:(String.equal "--smoke"))
