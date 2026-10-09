open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Editor = Gpuio_eio.Text_input
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module L = Gpuio_bonsai.Virtual_list
module Model = Stream_model
module Probe = Gpuio_performance_probe

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let perform scope ui_effect =
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
;;

let component ~model ~command ~events ~observed ~peak_active window graph =
  let open B.Let_syntax in
  let model = B.Expert.Var.value model in
  let history =
    L.component
      (module Int)
      (B.map model ~f:Model.rows)
      ~row_key:(fun id -> Key.of_string (Int.to_string id) |> ok)
      ~config:
        (L.Config.create
           ~height:(Estimated 160.)
           ~overscan:200.
           ~max_active:32
           ~scroll:Follow_tail_when_at_end
           ()
         |> ok)
      ~style:(B.return (style [ Width (px 1168.); Height (px 600.); Shrink 0. ]))
      ~render_row:(fun ~key:_ ~data ~lifetime:_ _graph ->
        let%arr row = data in
        V.text
          ~style:(style [ Padding (px 8.); Font_size 14.; Line_height (px 20.) ])
          (sprintf "Stream %d · block %d\n%s" row.Model.Row.stream row.block row.text))
      graph
  in
  let editor =
    Editor.create
      window
      ~config:
        (B.return
           (Text_input.Config.create
              ~mode:Multiline
              ~label:"Qualification composer"
              ~min_rows:2
              ~max_rows:3
              ()
            |> ok))
      graph
  in
  B.Edge.after_display
    (let%arr editor = editor
     and history = history in
     E.of_thunk (fun () ->
       let history = ok history in
       assert (not (L.Output.budget_exhausted history));
       peak_active := Int.max !peak_active (L.Output.active_rows history);
       observed := Some (editor, history)))
    graph;
  let%arr editor = editor
  and history = history
  and sequence, request = B.Expert.Var.value command in
  V.column
    ~style:
      (style
         [ Width (px 1200.)
         ; Height (px 800.)
         ; Padding (px 16.)
         ; Gap (px 8.)
         ; Background (Background.solid (Color.rgb_exn 0xf5f6f8))
         ; Foreground (Color.rgb_exn 0x182030)
         ])
    [ V.text "GPUIO qualification · four streams and native typing"
    ; L.Output.view (ok history)
    ; Editor.view ~style:(style [ Width (px 1168.); Height (px 80.); Shrink 0. ]) editor
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
  let seconds = if smoke then 4 else 120 in
  let updates = seconds * 20 in
  let keys = seconds * 10 in
  App.run ~exit_on_last_window:false (fun env app ->
    let scope = App.scope app in
    let clock = Eio.Stdenv.clock env in
    let mono = Eio.Stdenv.mono_clock env in
    let input = Eio.Buf_read.of_flow (Eio.Stdenv.stdin env) ~max_size:128 in
    let model = B.Expert.Var.create (Model.create ()) in
    let command = B.Expert.Var.create (1L, Probe.Command.Begin) in
    let events = Queue.create () in
    let observed = ref None in
    let peak_active = ref 0 in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO · Streaming and typing qualification"
        ~width:1200.
        ~height:800.
        (component ~model ~command ~events ~observed ~peak_active)
      |> ok
    in
    let emit name value =
      Eio.Flow.copy_string
        ("GPUIO_STREAM_PERF " ^ name ^ " " ^ Sexp.to_string value ^ "\n")
        (Eio.Stdenv.stdout env)
    in
    let sync f = perform scope (E.of_thunk f) in
    let wait label f =
      try
        Eio.Time.with_timeout_exn clock 30. (fun () ->
          while not (f ()) do
            Eio.Time.sleep clock 0.005
          done)
      with
      | Eio.Time.Timeout -> failwith ("Timed out: " ^ label)
    in
    let next_event () =
      wait "native probe" (fun () -> not (Queue.is_empty events));
      Queue.dequeue_exn events
    in
    let sequence = ref 1L in
    let send request =
      sequence := Int64.succ !sequence;
      sync (fun () -> B.Expert.Var.set command (!sequence, request))
    in
    let read_line () =
      Eio.Time.with_timeout_exn clock 30. (fun () -> Eio.Buf_read.line input)
    in
    let editor () = fst (Option.value_exn !observed) in
    let frame () =
      let promise, resolver = Eio.Promise.create () in
      App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
        E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
      |> ok;
      Eio.Time.with_timeout_exn clock 30. (fun () -> Eio.Promise.await promise)
    in
    let work () =
      emit "config" [%sexp (smoke : bool), (seconds : int), (updates : int), (keys : int)];
      wait "composer" (fun () -> Option.is_some !observed);
      (match next_event () with
       | Probe.Event.Begun ns -> emit "begin" [%sexp ("streams" : string), (ns : int64)]
       | event -> raise_s [%sexp (event : Probe.Event.t)]);
      emit "ready" [%sexp (keys : int)];
      assert (String.equal (read_line ()) "go");
      let started = Eio.Time.Mono.now mono in
      let elapsed () =
        Mtime.span started (Eio.Time.Mono.now mono) |> Mtime.Span.to_uint64_ns
      in
      let deadline seconds =
        Mtime.add_span
          started
          (Mtime.Span.of_float_ns (seconds *. 1e9) |> Option.value_exn)
        |> Option.value_exn
      in
      let times = Array.init 4 ~f:(fun _ -> Array.create ~len:updates 0L) in
      Eio.Switch.run (fun sw ->
        for stream = 0 to 3 do
          Eio.Fiber.fork ~sw (fun () ->
            for index = 1 to updates do
              Eio.Time.Mono.sleep_until mono (deadline (Float.of_int index /. 20.));
              sync (fun () ->
                let next =
                  Model.append (B.Expert.Var.get model) ~stream ~sequence:index |> ok
                in
                B.Expert.Var.set model next);
              times.(stream).(index - 1) <- elapsed ()
            done)
        done;
        for second = 1 to seconds do
          Eio.Time.Mono.sleep_until mono (deadline (Float.of_int second));
          let progress, typed =
            sync (fun () ->
              let typed =
                Editor.snapshot (editor ())
                |> Option.value_map ~default:0 ~f:(fun snapshot ->
                  String.length (Text_input.Snapshot.text snapshot))
              in
              Model.progress (B.Expert.Var.get model), typed)
          in
          emit
            "progress"
            [%sexp
              (second : int)
            , (elapsed () : int64)
            , (progress : Model.Progress.t list)
            , (typed : int)]
        done);
      sync (fun () ->
        Model.validate (B.Expert.Var.get model) ~updates_per_stream:updates |> ok);
      Array.iteri times ~f:(fun stream times ->
        emit "ui-updates" [%sexp (stream : int), (times : int64 array)]);
      emit "streams-complete" [%sexp (elapsed () : int64), (!peak_active : int)];
      assert (String.equal (read_line ()) "typed");
      let snapshot =
        perform scope (Editor.read_snapshot (editor ()))
        |> Result.map_error ~f:Text_input.Command_error.sexp_of_t
        |> function
        | Ok snapshot -> snapshot
        | Error error -> raise_s error
      in
      let expected = String.init keys ~f:(fun index -> "asdf".[index % 4]) in
      assert (String.equal (Text_input.Snapshot.text snapshot) expected);
      emit
        "typed"
        [%sexp
          (String.length expected : int)
        , (Text_input.Snapshot.revision snapshot |> Text_input.Revision.to_int64 : int64)];
      frame ();
      send Probe.Command.Finish;
      let finished = next_event () in
      let counts =
        match finished with
        | Probe.Event.Finished { counts; _ } -> counts
        | event -> raise_s [%sexp (event : Probe.Event.t)]
      in
      emit "finish" [%sexp ("streams" : string), (finished : Probe.Event.t)];
      List.iteri counts ~f:(fun metric _ ->
        let offset = ref 0 in
        let more = ref true in
        while !more do
          send (Probe.Command.Buckets { metric; offset = !offset });
          let event = next_event () in
          (match event with
           | Probe.Event.Buckets page ->
             assert (page.metric = metric && page.offset = !offset);
             offset := !offset + List.length page.values;
             assert (!offset <= page.total);
             more := !offset < page.total
           | event -> raise_s [%sexp (event : Probe.Event.t)]);
          emit "buckets" [%sexp ("streams" : string), (event : Probe.Event.t)]
        done);
      for stream = 0 to 3 do
        let key = ((updates - 1) / Model.fragments_per_block * 4) + stream in
        let history = snd (Option.value_exn !observed) in
        perform scope (L.Controller.scroll_to (L.Output.controller history) key |> ok);
        frame ();
        emit "verify-row" [%sexp (stream : int), (updates : int)];
        assert (String.equal (read_line ()) (sprintf "verified %d" stream))
      done;
      sync (fun () ->
        App.Window.close window;
        observed := None;
        B.Expert.Var.set model (Model.create ()));
      wait "retirement" (fun () ->
        let d = App.diagnostics app in
        d.windows = 0
        && d.pending_requests = 0
        && d.queued_commands = 0
        && d.queued_jobs = 0);
      emit "cleanup" [%sexp (App.diagnostics app : App.Diagnostics.t)];
      emit "complete" [%sexp (updates * 4 * Model.fragment_bytes : int)];
      App.shutdown app
    in
    ignore
      (Scope.start scope ~f:work ~on_result:(fun result ->
         E.of_thunk (fun () -> ok result))
       |> ok
       : Scope.Task.t))
;;

let () = run ~smoke:(Array.exists (Sys.get_argv ()) ~f:(String.equal "--smoke"))
